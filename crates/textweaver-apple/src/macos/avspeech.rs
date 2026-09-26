//! The `avspeech` backend: `AVSpeechSynthesizer` writing into buffers, with
//! textweaver playing the audio.
//!
//! `write(_:toBufferCallback:)` hands over PCM buffers while the delegate's
//! `willSpeakRangeOfSpeechString` fires for each word, interleaved with the
//! buffers (probe 3), so the number of samples received when a word callback
//! runs is that word's offset in the audio. The backend appends each buffer
//! to its output with a counter of samples played, and emits a word's
//! `Word { audio_ms }` when playback reaches the word: highlighting follows
//! the audio clock, and pause and resume are exact because textweaver owns
//! playback.
//!
//! The synthesizer is created and driven on the speech thread, but it
//! delivers its buffers and callbacks through the main dispatch queue
//! (probe 5): the application's main thread must run its run loop
//! ([`crate::run_main_loop_until`] or [`crate::pump_main_loop`]). Without
//! that, speech never starts; after five seconds the utterance ends with an
//! error that says so.
//!
//! Rate maps through [`crate::rate`]'s measured tables to the utterance's
//! `0.0..=1.0` rate; pitch is a multiplier (`2^(semitones/12)`); volume is
//! applied by the output, immediately.

use std::collections::VecDeque;
use std::ops::Range;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AnyThread, DefinedClass, define_class, msg_send};
use objc2_avf_audio::{
    AVAudioBuffer, AVAudioCommonFormat, AVAudioPCMBuffer, AVSpeechBoundary, AVSpeechSynthesisVoice,
    AVSpeechSynthesisVoiceGender, AVSpeechSynthesizer, AVSpeechSynthesizerDelegate,
    AVSpeechUtterance,
};
use objc2_foundation::{NSRange, NSString};
use textweaver_core::{Utterance, UtteranceId};
use textweaver_speech::{
    BackendId, Caps, EventSink, RawEvent, SpeechBackend, SpeechError, Voice, VoiceParams,
};

use super::output::{AudioOut, RodioOut, SilentOut};
use super::runloop;
use crate::audio::Pcm;
use crate::range::WordTracker;
use crate::{rate, voices};

const ID: BackendId = crate::AVSPEECH_ID;

/// How long to wait for the first buffer before concluding that the main
/// run loop is not running.
const FIRST_BUFFER_TIMEOUT: Duration = Duration::from_secs(5);

/// How long to wait for the other end signal once one of the two (the
/// empty buffer, the delegate's finish) has arrived.
const FINISH_GRACE: Duration = Duration::from_millis(500);

/// Synthesis whose synthesizer says it is no longer speaking, which has
/// reported the text's last word, and which has delivered nothing for this
/// long, is finished.
const IDLE_GRACE: Duration = Duration::from_millis(300);

/// As `IDLE_GRACE`, when the last word has not been reported.
const IDLE_FALLBACK: Duration = Duration::from_millis(1500);

/// Synthesis that delivers nothing for this long after it started
/// producing audio is treated as finished.
const STALL_TIMEOUT: Duration = Duration::from_secs(3);

/// How the end of a synthesis was recognized. On macOS 15 and 26 both end
/// signals arrive; on macOS 14 the backend may have to rely on the others.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SynthesisEnd {
    /// The empty final buffer and the delegate's finish both arrived.
    #[default]
    Both,
    /// Only the empty final buffer arrived.
    EmptyBuffer,
    /// Only the delegate's finish arrived.
    Delegate,
    /// The synthesizer stopped speaking and delivered nothing more.
    Idle,
    /// Nothing arrived for three seconds.
    Stalled,
}

/// The error reported when the synthesizer's callbacks never arrive.
const NO_MAIN_LOOP: &str = "AVSpeechSynthesizer produced no audio: its callbacks need the main \
     thread's run loop (textweaver_apple::run_main_loop_until or pump_main_loop)";

/// Speech synthesized into memory by [`AvSpeechBackend::synthesize_words`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Synthesis {
    /// The audio (mono).
    pub pcm: Pcm,
    /// Each written word's byte range in the text and its audio offset in
    /// milliseconds, in order.
    pub words: Vec<(Range<u32>, u32)>,
    /// How many buffers the synthesizer delivered (its reporting
    /// granularity: a word's offset is exact to within one buffer).
    pub buffers: usize,
    /// How the end of synthesis was recognized.
    pub end: SynthesisEnd,
}

/// Where the `avspeech` backend's audio goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Output {
    /// The default output device.
    Speakers,
    /// Nowhere: samples are consumed by the clock, in real time or at once.
    /// Word events and their timing are the same as with `Speakers`. For
    /// tests and measurements; nothing is ever played aloud.
    Silent {
        /// Consume samples at their sample rate (true) or at once.
        realtime: bool,
    },
}

/// What the synthesizer's callbacks have delivered for the current job.
#[derive(Default)]
struct Shared {
    /// Token of the job being synthesized; 0 when none.
    job: u64,
    /// Address of that job's `AVSpeechUtterance`, to match delegate calls.
    utterance: usize,
    /// Samples received so far for the job.
    samples: u64,
    /// Sample rate of the job's audio.
    rate: u32,
    /// Chunks received and not yet taken.
    audio: Vec<Vec<f32>>,
    /// Word callbacks: (sample offset, UTF-16 location, UTF-16 length).
    words: Vec<(u64, usize, usize)>,
    /// The empty buffer that ends synthesis has arrived.
    buffers_done: bool,
    /// When it arrived.
    buffers_done_at: Option<Instant>,
    /// The delegate reported the utterance finished.
    delegate_done: bool,
    /// When it did.
    delegate_done_at: Option<Instant>,
    /// When the first buffer arrived.
    first_buffer: Option<Instant>,
    /// When the last buffer or word callback arrived.
    last_activity: Option<Instant>,
    /// Non-empty buffers received.
    buffers: usize,
    /// Buffer formats this backend cannot read.
    error: Option<String>,
}

impl Shared {
    /// Whether synthesis of the job is over, and how that was recognized:
    /// both end signals arrived, or one did `FINISH_GRACE` ago, or (once
    /// audio has started) the synthesizer is no longer speaking and nothing
    /// arrived for `IDLE_GRACE` after the last word (`IDLE_FALLBACK`
    /// without it), or nothing arrived for `STALL_TIMEOUT`. macOS 14 sends
    /// its word callbacks after the buffers and neither end signal, so the
    /// last word matters there.
    fn complete(&self, now: Instant, speaking: bool, last_word: bool) -> Option<SynthesisEnd> {
        let since = |t: Option<Instant>| t.map(|t| now.saturating_duration_since(t));
        let quiet = since(self.last_activity).unwrap_or_default();
        if self.buffers_done && self.delegate_done {
            Some(SynthesisEnd::Both)
        } else if since(self.buffers_done_at).is_some_and(|d| d >= FINISH_GRACE) {
            Some(SynthesisEnd::EmptyBuffer)
        } else if since(self.delegate_done_at).is_some_and(|d| d >= FINISH_GRACE) {
            Some(SynthesisEnd::Delegate)
        } else if self.first_buffer.is_none() {
            None
        } else if !speaking && quiet >= if last_word { IDLE_GRACE } else { IDLE_FALLBACK } {
            Some(SynthesisEnd::Idle)
        } else if quiet >= STALL_TIMEOUT {
            Some(SynthesisEnd::Stalled)
        } else {
            None
        }
    }

    fn end_buffers(&mut self) {
        self.buffers_done = true;
        self.buffers_done_at.get_or_insert_with(Instant::now);
    }
}

fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

/// Copies the first channel of a PCM buffer as `f32` samples.
fn read_samples(pcm: &AVAudioPCMBuffer) -> Result<(Vec<f32>, u32), String> {
    // SAFETY: plain property reads on a live buffer handed to the callback.
    let (frames, format) = unsafe { (pcm.frameLength() as usize, pcm.format()) };
    // SAFETY: property reads on the buffer's live format.
    let (common, rate, channels, interleaved) = unsafe {
        (
            format.commonFormat(),
            format.sampleRate(),
            format.channelCount() as usize,
            format.isInterleaved(),
        )
    };
    let step = if interleaved { channels.max(1) } else { 1 };
    let rate = rate.round() as u32;
    if frames == 0 {
        return Ok((Vec::new(), rate));
    }
    let samples = if common == AVAudioCommonFormat::PCMFormatFloat32 {
        // SAFETY: for a float32 buffer `floatChannelData` points to one
        // pointer per channel (one in all when interleaved), each to at least
        // `frames * step` samples, valid while the buffer is.
        unsafe {
            let chans = pcm.floatChannelData();
            if chans.is_null() {
                return Err("float buffer without data".into());
            }
            let data = (*chans).as_ptr();
            let all = std::slice::from_raw_parts(data, frames * step);
            all.iter().step_by(step).copied().collect()
        }
    } else if common == AVAudioCommonFormat::PCMFormatInt16 {
        // SAFETY: as above, for 16-bit integer buffers.
        unsafe {
            let chans = pcm.int16ChannelData();
            if chans.is_null() {
                return Err("int16 buffer without data".into());
            }
            let data = (*chans).as_ptr();
            let all = std::slice::from_raw_parts(data, frames * step);
            all.iter()
                .step_by(step)
                .map(|&s| f32::from(s) / 32768.0)
                .collect()
        }
    } else {
        return Err(format!("unsupported sample format {}", common.0));
    };
    Ok((samples, rate))
}

/// Instance variables of the delegate.
struct DelegateIvars {
    shared: Arc<Mutex<Shared>>,
}

define_class!(
    // SAFETY:
    // - NSObject has no subclassing requirements.
    // - AvDelegate does not implement Drop.
    #[unsafe(super(NSObject))]
    #[name = "TextweaverAvSpeechDelegate"]
    #[ivars = DelegateIvars]
    struct AvDelegate;

    impl AvDelegate {
        // SAFETY: the signature matches the delegate method.
        #[unsafe(method(speechSynthesizer:willSpeakRangeOfSpeechString:utterance:))]
        fn will_speak_range(
            &self,
            _synth: &AVSpeechSynthesizer,
            range: NSRange,
            utterance: &AVSpeechUtterance,
        ) {
            let mut s = lock(&self.ivars().shared);
            if s.job != 0 && s.utterance == utterance as *const AVSpeechUtterance as usize {
                let at = s.samples;
                s.words.push((at, range.location, range.length));
                s.last_activity = Some(Instant::now());
            }
        }

        // SAFETY: the signature matches the delegate method.
        #[unsafe(method(speechSynthesizer:didFinishSpeechUtterance:))]
        fn did_finish(&self, _synth: &AVSpeechSynthesizer, utterance: &AVSpeechUtterance) {
            let mut s = lock(&self.ivars().shared);
            if s.job != 0 && s.utterance == utterance as *const AVSpeechUtterance as usize {
                s.delegate_done = true;
                s.delegate_done_at.get_or_insert_with(Instant::now);
            }
        }
    }

    unsafe impl NSObjectProtocol for AvDelegate {}

    // SAFETY: every method of the protocol is optional; the ones implemented
    // above have the protocol's signatures.
    unsafe impl AVSpeechSynthesizerDelegate for AvDelegate {}
);

impl AvDelegate {
    fn new(shared: Arc<Mutex<Shared>>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(DelegateIvars { shared });
        // SAFETY: NSObject's designated initializer.
        unsafe { msg_send![super(this), init] }
    }
}

/// A synthesizer with its delegate and callback state.
struct Synth {
    synth: Retained<AVSpeechSynthesizer>,
    _delegate: Retained<AvDelegate>,
    shared: Arc<Mutex<Shared>>,
}

impl Synth {
    fn new() -> Self {
        let shared: Arc<Mutex<Shared>> = Arc::default();
        let delegate = AvDelegate::new(shared.clone());
        // SAFETY: creating a synthesizer has no preconditions.
        let synth = unsafe { AVSpeechSynthesizer::new() };
        // SAFETY: the delegate is retained by `Synth` for as long as the
        // synthesizer (the property is weak).
        unsafe { synth.setDelegate(Some(ProtocolObject::from_ref(&*delegate))) };
        Synth {
            synth,
            _delegate: delegate,
            shared,
        }
    }

    /// Starts writing `utt` into buffers as job `token`.
    fn write(&self, utt: &AVSpeechUtterance, token: u64) {
        {
            let mut s = lock(&self.shared);
            *s = Shared {
                job: token,
                utterance: utt as *const AVSpeechUtterance as usize,
                ..Shared::default()
            };
        }
        let shared = self.shared.clone();
        let block = RcBlock::new(move |buf: NonNull<AVAudioBuffer>| {
            // SAFETY: the synthesizer passes a live buffer for the duration
            // of the callback.
            let buf: &AVAudioBuffer = unsafe { buf.as_ref() };
            let mut s = lock(&shared);
            if s.job != token {
                return;
            }
            let Some(pcm) = buf.downcast_ref::<AVAudioPCMBuffer>() else {
                return;
            };
            match read_samples(pcm) {
                Ok((samples, _)) if samples.is_empty() => s.end_buffers(),
                Ok((samples, rate)) => {
                    let now = Instant::now();
                    s.first_buffer.get_or_insert(now);
                    s.last_activity = Some(now);
                    s.buffers += 1;
                    s.rate = rate;
                    s.samples += samples.len() as u64;
                    s.audio.push(samples);
                }
                Err(e) => {
                    s.error = Some(e);
                    s.end_buffers();
                }
            }
        });
        // SAFETY: the block is a valid buffer callback; the synthesizer
        // copies it.
        unsafe {
            self.synth
                .writeUtterance_toBufferCallback(utt, RcBlock::as_ptr(&block));
        }
    }

    fn speaking(&self) -> bool {
        // SAFETY: a property read.
        unsafe { self.synth.isSpeaking() }
    }

    fn stop(&self) {
        lock(&self.shared).job = 0;
        // SAFETY: stopping has no preconditions.
        unsafe {
            let _ = self
                .synth
                .stopSpeakingAtBoundary(AVSpeechBoundary::Immediate);
        }
    }
}

/// One utterance from synthesis to the end of its playback.
struct Job {
    id: UtteranceId,
    text: String,
    tracker: WordTracker,
    token: u64,
    _utt: Retained<AVSpeechUtterance>,
    /// Samples consumed by the output so far.
    played: Arc<AtomicU64>,
    /// Samples appended to the output.
    appended: u64,
    rate: u32,
    synth_done: bool,
    /// Words not yet reported: (sample offset, byte range).
    words: VecDeque<(u64, Range<u32>)>,
    started: bool,
    write_started: Instant,
}

/// `AVSpeechSynthesizer` into buffers, played by textweaver. See the module
/// docs.
pub struct AvSpeechBackend {
    synth: Synth,
    out: Box<dyn AudioOut>,
    params: VoiceParams,
    voice: Option<String>,
    voice_obj: Option<Retained<AVSpeechSynthesisVoice>>,
    pending: VecDeque<Utterance>,
    jobs: VecDeque<Job>,
    deferred: Vec<(UtteranceId, RawEvent)>,
    next_token: u64,
    first_audio_latency: Option<Duration>,
    word_offsets: Vec<u32>,
    raw_rate: Option<f32>,
    last_end: SynthesisEnd,
}

fn find_voice(id: &str) -> Option<Retained<AVSpeechSynthesisVoice>> {
    // SAFETY: a class method with a valid string argument.
    unsafe { AVSpeechSynthesisVoice::voiceWithIdentifier(&NSString::from_str(id)) }
}

fn voice_ids() -> Vec<String> {
    // SAFETY: class method and property reads on returned voices.
    unsafe {
        AVSpeechSynthesisVoice::speechVoices()
            .iter()
            .map(|v| v.identifier().to_string())
            .collect()
    }
}

impl AvSpeechBackend {
    /// Creates the backend (on the speech thread), playing to `output`.
    pub fn new(output: Output) -> Result<Self, SpeechError> {
        let out: Box<dyn AudioOut> = match output {
            Output::Speakers => {
                Box::new(RodioOut::open().map_err(|e| SpeechError::Unavailable(ID, e))?)
            }
            Output::Silent { realtime } => Box::new(SilentOut::new(realtime)),
        };
        let ids = voice_ids();
        if ids.is_empty() {
            return Err(SpeechError::Unavailable(ID, "no system voices".into()));
        }
        let voice = voices::default_voice(ids.iter().map(String::as_str)).map(str::to_string);
        let voice_obj = voice.as_deref().and_then(find_voice);
        Ok(AvSpeechBackend {
            synth: Synth::new(),
            out,
            params: VoiceParams::default(),
            voice,
            voice_obj,
            pending: VecDeque::new(),
            jobs: VecDeque::new(),
            deferred: Vec::new(),
            next_token: 1,
            first_audio_latency: None,
            word_offsets: Vec::new(),
            raw_rate: None,
            last_end: SynthesisEnd::Both,
        })
    }

    /// The voice in use (the requested one, or the default, Eloquence Reed
    /// when installed; `None` is the system default voice).
    pub fn voice(&self) -> Option<&str> {
        self.voice.as_deref()
    }

    /// Time from starting synthesis of the most recent utterance to its
    /// first audio buffer.
    pub fn first_audio_latency(&self) -> Option<Duration> {
        self.first_audio_latency
    }

    /// How the end of the most recent synthesis was recognized.
    pub fn last_synthesis_end(&self) -> SynthesisEnd {
        self.last_end
    }

    /// Bypasses the rate table and sets `AVSpeechUtterance.rate` directly
    /// (`0.0..=1.0`; `None` restores the table). For calibration
    /// measurements.
    pub fn set_raw_rate(&mut self, utterance_rate: Option<f32>) {
        self.raw_rate = utterance_rate.map(|r| r.clamp(0.0, 1.0));
    }

    /// Audio offsets (ms) of the words of the most recently started
    /// utterance, in order.
    pub fn last_word_offsets(&self) -> &[u32] {
        &self.word_offsets
    }

    fn utterance_for(&self, text: &str) -> Retained<AVSpeechUtterance> {
        let table = rate::av_table(self.voice.as_deref());
        // SAFETY: creating and configuring an utterance has no
        // preconditions beyond valid arguments.
        unsafe {
            let utt = AVSpeechUtterance::speechUtteranceWithString(&NSString::from_str(text));
            utt.setVoice(self.voice_obj.as_deref());
            utt.setRate(
                self.raw_rate
                    .unwrap_or_else(|| table.engine_value(self.params.rate.clamped().wpm())),
            );
            utt.setPitchMultiplier(rate::pitch_multiplier(
                self.params.pitch.clamped().semitones(),
            ));
            utt.setVolume(1.0);
            utt.setPrefersAssistiveTechnologySettings(false);
            utt
        }
    }

    fn flush_deferred(&mut self, sink: &mut dyn EventSink) {
        for (id, ev) in self.deferred.drain(..) {
            sink.emit(id, ev);
        }
    }

    fn synthesizing(&self) -> bool {
        self.jobs.iter().any(|j| !j.synth_done)
    }

    /// Starts synthesizing the next pending utterance when idle.
    fn start_next(&mut self, sink: &mut dyn EventSink) {
        while !self.synthesizing() {
            let Some(u) = self.pending.pop_front() else {
                return;
            };
            if !sink.is_current(u.id) {
                sink.emit(u.id, RawEvent::Cancelled);
                continue;
            }
            let token = self.next_token;
            self.next_token += 1;
            let utt = self.utterance_for(&u.text);
            let empty = u.text.trim().is_empty();
            if !empty {
                self.synth.write(&utt, token);
            }
            self.jobs.push_back(Job {
                id: u.id,
                tracker: WordTracker::new(&u.text),
                text: u.text,
                token,
                _utt: utt,
                played: Arc::default(),
                appended: 0,
                rate: 0,
                synth_done: empty,
                words: VecDeque::new(),
                started: false,
                write_started: Instant::now(),
            });
        }
    }

    /// Moves what the callbacks delivered into the job written last, which
    /// keeps collecting after its synthesis is recognized as finished, until
    /// the next job's write starts (late word callbacks on macOS 14).
    fn collect(&mut self, sink: &mut dyn EventSink) {
        let token = lock(&self.synth.shared).job;
        let Some(job) = self.jobs.iter_mut().find(|j| j.token == token) else {
            return;
        };
        let now = Instant::now();
        let speaking = self.synth.speaking();
        let (audio, words, rate, first, error) = {
            let mut s = lock(&self.synth.shared);
            if s.job != job.token {
                return;
            }
            (
                std::mem::take(&mut s.audio),
                std::mem::take(&mut s.words),
                s.rate,
                s.first_buffer,
                s.error.take(),
            )
        };
        if rate > 0 {
            job.rate = rate;
        }
        for (at, loc, len) in words {
            if let Some(range) = job.tracker.word(&job.text, loc, len) {
                job.words.push_back((at, range));
            }
        }
        let had_audio = job.appended > 0;
        for chunk in audio {
            job.appended += chunk.len() as u64;
            self.out.append(chunk, job.rate, job.played.clone());
        }
        if let (false, Some(first)) = (had_audio, first) {
            self.first_audio_latency = Some(first.saturating_duration_since(job.write_started));
        }
        if let Some(e) = error {
            sink.emit(job.id, RawEvent::Error(e));
        }
        if job.synth_done {
            return;
        }
        let last_word = job.tracker.reached_end(&job.text);
        let complete = lock(&self.synth.shared).complete(now, speaking, last_word);
        if let Some(end) = complete {
            job.synth_done = true;
            self.last_end = end;
        } else if first.is_none() && now.duration_since(job.write_started) >= FIRST_BUFFER_TIMEOUT {
            sink.emit(job.id, RawEvent::Error(NO_MAIN_LOOP.into()));
            job.synth_done = true;
            self.synth.stop();
            self.synth = Synth::new();
        }
    }

    /// Reports progress of the job at the head of playback.
    fn report(&mut self, sink: &mut dyn EventSink) {
        while let Some(job) = self.jobs.front_mut() {
            let played = job.played.load(Ordering::Relaxed);
            if !job.started && (played > 0 || (job.synth_done && job.appended == 0)) {
                job.started = true;
                sink.emit(job.id, RawEvent::Started);
                self.word_offsets.clear();
            }
            if !job.started {
                return;
            }
            let finished = job.synth_done && played >= job.appended;
            while let Some((at, _)) = job.words.front() {
                if *at > played && !finished {
                    break;
                }
                let Some((at, range)) = job.words.pop_front() else {
                    break;
                };
                let ms = if job.rate > 0 {
                    u32::try_from(at * 1000 / u64::from(job.rate)).unwrap_or(u32::MAX)
                } else {
                    0
                };
                self.word_offsets.push(ms);
                sink.emit(
                    job.id,
                    RawEvent::Word {
                        byte_range: range,
                        audio_ms: Some(ms),
                    },
                );
            }
            if !finished {
                return;
            }
            sink.emit(job.id, RawEvent::Finished);
            self.jobs.pop_front();
        }
    }

    /// Synthesizes `text` into memory with each word's audio offset,
    /// waiting for the synthesizer (whose callbacks need the main run loop;
    /// on the main thread this runs it). Volume is applied to the samples.
    /// Useful for exports with word timing (subtitles, audiobooks).
    pub fn synthesize_words(&mut self, text: &str) -> Result<Synthesis, SpeechError> {
        let synth = Synth::new();
        let utt = self.utterance_for(text);
        let token = u64::MAX;
        synth.write(&utt, token);
        let started = Instant::now();
        let mut tracker = WordTracker::new(text);
        let mut samples = Vec::new();
        let mut words = Vec::new();
        let (rate, buffers, end) = loop {
            if runloop::is_main_thread() {
                runloop::run_current_once(Duration::from_millis(5));
            } else {
                std::thread::sleep(Duration::from_millis(5));
            }
            let speaking = synth.speaking();
            let mut s = lock(&synth.shared);
            for chunk in s.audio.drain(..) {
                samples.extend_from_slice(&chunk);
            }
            for (at, loc, len) in s.words.drain(..) {
                if let Some(range) = tracker.word(text, loc, len) {
                    words.push((range, at));
                }
            }
            if let Some(e) = s.error.take() {
                return Err(SpeechError::Engine(e));
            }
            if let Some(end) = s.complete(Instant::now(), speaking, tracker.reached_end(text)) {
                break (s.rate, s.buffers, end);
            } else if s.first_buffer.is_none() && started.elapsed() >= FIRST_BUFFER_TIMEOUT {
                drop(s);
                synth.stop();
                return Err(SpeechError::Engine(NO_MAIN_LOOP.into()));
            }
        };
        let volume = self.params.volume.fraction();
        if volume < 1.0 {
            samples.iter_mut().for_each(|s| *s *= volume);
        }
        let words = words
            .into_iter()
            .map(|(range, at)| {
                let ms = if rate > 0 {
                    at * 1000 / u64::from(rate)
                } else {
                    0
                };
                (range, u32::try_from(ms).unwrap_or(u32::MAX))
            })
            .collect();
        Ok(Synthesis {
            pcm: Pcm {
                sample_rate: rate,
                channels: 1,
                samples,
            },
            words,
            buffers,
            end,
        })
    }
}

impl SpeechBackend for AvSpeechBackend {
    fn id(&self) -> BackendId {
        ID
    }

    fn capabilities(&self) -> Caps {
        Caps::WORD_EVENTS
            | Caps::AUDIO_CLOCK
            | Caps::PAUSE
            | Caps::PITCH
            | Caps::VOLUME
            | Caps::SYNTH_TO_FILE
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        // SAFETY: class method and property reads on the returned voices.
        let mut out: Vec<Voice> = unsafe {
            AVSpeechSynthesisVoice::speechVoices()
                .iter()
                .map(|v| {
                    let gender = match v.gender() {
                        AVSpeechSynthesisVoiceGender::Male => Some("male"),
                        AVSpeechSynthesisVoiceGender::Female => Some("female"),
                        _ => None,
                    };
                    voices::voice(
                        &v.identifier().to_string(),
                        &v.name().to_string(),
                        &v.language().to_string(),
                        gender,
                    )
                })
                .collect()
        };
        voices::sort_voices(&mut out);
        Ok(out)
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        let (voice, voice_obj) = match &params.voice {
            Some(id) => {
                let obj = find_voice(id).ok_or_else(|| SpeechError::UnknownVoice(id.clone()))?;
                (Some(id.clone()), Some(obj))
            }
            None => {
                let v = voices::default_voice(voice_ids().iter().map(String::as_str))
                    .map(str::to_string);
                let obj = v.as_deref().and_then(find_voice);
                (v, obj)
            }
        };
        self.params = params.clone();
        self.voice = voice;
        self.voice_obj = voice_obj;
        self.out.set_volume(self.params.volume.fraction());
        Ok(())
    }

    fn effective_wpm(&self) -> u16 {
        rate::av_table(self.voice.as_deref()).effective_wpm(self.params.rate.clamped().wpm())
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        self.flush_deferred(sink);
        self.pending.push_back(utterance.clone());
        self.start_next(sink);
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.flush_deferred(sink);
        if runloop::is_main_thread() {
            runloop::run_current_once(Duration::ZERO);
        }
        self.out.tick();
        self.collect(sink);
        self.start_next(sink);
        self.report(sink);
    }

    fn stop(&mut self) {
        if self.synthesizing() {
            self.synth.stop();
            self.synth = Synth::new();
        }
        self.out.clear();
        self.out.resume();
        for job in self.jobs.drain(..) {
            self.deferred.push((job.id, RawEvent::Cancelled));
        }
        for u in self.pending.drain(..) {
            self.deferred.push((u.id, RawEvent::Cancelled));
        }
    }

    fn pause(&mut self) -> Result<(), SpeechError> {
        self.out.pause();
        Ok(())
    }

    fn resume(&mut self) -> Result<(), SpeechError> {
        self.out.resume();
        Ok(())
    }

    fn synthesize_to_file(&mut self, text: &str, path: &Path) -> Result<(), SpeechError> {
        let synthesis = self.synthesize_words(text)?;
        crate::audio::write_wav(path, &synthesis.pcm).map_err(|e| SpeechError::Io(e.to_string()))
    }
}
