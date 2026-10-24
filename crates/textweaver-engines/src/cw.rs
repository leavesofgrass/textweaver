//! CW (Morse code) as a speech engine.
//!
//! The engine sends the text as a tone in Morse code, character by
//! character, exactly as written (it declares [`Caps::RAW_TEXT`], so the
//! speech service normalizes nothing). `textweaver-cw` schedules and
//! renders each utterance at once, and the shared [`Playback`] client
//! plays it: the word events come from the schedule, so the highlight and
//! RSVP follow the word being sent to the sample, and they stop with the
//! audio when it pauses. Structural pauses are silence in the same feed.
//! Audio export writes the same samples, with the same word times.
//!
//! CW is chosen, never fallen back to: its registry entry is opt-in, so
//! the rule that textweaver always speaks if any engine can never lands
//! on Morse code.
//!
//! The options ([`CwOptions`], from `[speech.cw]`) are shared by the
//! process and read at each utterance, so a new speed or tone applies
//! from the next sentence without restarting speech ([`set_options`]).
//! Pitch keys move the tone by semitones; the volume keys scale it.
//!
//! As each character starts to sound, it is recorded in
//! [`textweaver_cw::sending()`] for the status line.

use std::collections::VecDeque;
use std::ops::Range;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use textweaver_core::{Utterance, UtteranceId};
use textweaver_cw::{Options, RenderOptions, Schedule, Timing, schedule, sending, to_samples};
use textweaver_enginehost::{AudioOutput, EndStatus, Playback, wav};
use textweaver_speech::{
    BackendId, BackendInfo, Caps, EventSink, FileSynthesis, RawEvent, SpeechBackend, SpeechError,
    Voice, VoiceParams, WordTiming,
};
use textweaver_store::Settings;

/// The engine's id: `[speech] backend = "cw"`.
pub const BACKEND_ID: BackendId = "cw";

/// The sample rate CW is rendered at.
pub const SAMPLE_RATE: u32 = 22_050;

/// What the engine can do.
pub const CAPS: Caps = Caps::WORD_EVENTS
    .union(Caps::AUDIO_CLOCK)
    .union(Caps::PLAYBACK_EVENTS)
    .union(Caps::PAUSE)
    .union(Caps::PITCH)
    .union(Caps::VOLUME)
    .union(Caps::SYNTH_TO_FILE)
    .union(Caps::SILENCE)
    .union(Caps::RAW_TEXT);

/// How CW sounds, from `[speech.cw]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CwOptions {
    /// Character speed, words per minute.
    pub wpm: u16,
    /// Effective (Farnsworth) speed, words per minute.
    pub effective_wpm: u16,
    /// The tone at the voice's own pitch, in hertz.
    pub tone_hz: u16,
    /// The tone's level, percent of full scale.
    pub volume: u8,
    /// Read prosigns written as `<BT>`.
    pub prosigns: bool,
}

impl Default for CwOptions {
    fn default() -> Self {
        CwOptions::from_settings(&Settings::default())
    }
}

impl CwOptions {
    /// The options `settings` give.
    pub fn from_settings(settings: &Settings) -> Self {
        let cw = &settings.speech.cw;
        CwOptions {
            wpm: cw.wpm,
            effective_wpm: cw.effective_wpm,
            tone_hz: cw.tone_hz,
            volume: cw.volume,
            prosigns: cw.prosigns,
        }
    }

    fn schedule_options(&self) -> Options {
        Options {
            timing: Timing::new(self.wpm, self.effective_wpm),
            prosigns: self.prosigns,
        }
    }
}

static OPTIONS: Mutex<Option<CwOptions>> = Mutex::new(None);

/// Sets the options every CW engine in the process uses from its next
/// utterance on. [`crate::speech_registry_for`] and the app call it when
/// `[speech.cw]` changes.
pub fn set_options(options: CwOptions) {
    *OPTIONS.lock().unwrap_or_else(|e| e.into_inner()) = Some(options);
}

/// The options in effect.
pub fn options() -> CwOptions {
    OPTIONS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or_default()
}

/// The registry entry: opt-in, always available.
pub fn backend_description() -> BackendInfo {
    BackendInfo {
        id: BACKEND_ID,
        name: "CW (Morse code)",
        priority: -100,
        opt_in: true,
        available: true,
        caps: CAPS,
    }
}

/// One utterance queued or sounding, for the status line.
#[derive(Debug)]
struct Queued {
    id: UtteranceId,
    /// Each character: its first sample, its label, and its word.
    chars: Vec<(u64, String, usize)>,
    /// Each word's byte range and first sample.
    words: Vec<(Range<u32>, u64)>,
    skipped: u32,
    /// The next character not yet recorded as sent.
    next: usize,
    /// The word of the last character recorded.
    last_word: Option<usize>,
    /// The feed position when a word was reached, and that word's sample.
    anchor: Option<(u64, u64)>,
}

/// Forwards events and keeps a copy, so the status line can follow them.
struct Tap<'a> {
    inner: &'a mut dyn EventSink,
    seen: Vec<(UtteranceId, RawEvent)>,
}

impl EventSink for Tap<'_> {
    fn emit(&mut self, id: UtteranceId, event: RawEvent) {
        self.seen.push((id, event.clone()));
        self.inner.emit(id, event);
    }

    fn is_current(&self, id: UtteranceId) -> bool {
        self.inner.is_current(id)
    }
}

/// The CW engine (see the module docs).
pub struct CwBackend {
    playback: Playback<()>,
    params: VoiceParams,
    /// The utterance whose audio is in but not yet ended: its end waits
    /// for a structural pause the service may ask for right after
    /// `speak`.
    open: Option<u64>,
    queued: VecDeque<Queued>,
}

impl std::fmt::Debug for CwBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CwBackend")
            .field("playback", &self.playback)
            .field("queued", &self.queued.len())
            .finish_non_exhaustive()
    }
}

impl CwBackend {
    /// An engine playing to `output` (the silent output in tests).
    pub fn new(output: AudioOutput) -> Self {
        CwBackend {
            playback: Playback::new(BACKEND_ID, output, SAMPLE_RATE),
            params: VoiceParams::default(),
            open: None,
            queued: VecDeque::new(),
        }
    }

    /// `text` scheduled and rendered with the current options and pitch.
    fn render(&self, text: &str) -> (Schedule, Vec<i16>) {
        let o = options();
        let s = schedule(text, &o.schedule_options());
        let semitones = f64::from(self.params.pitch.semitones());
        let render = RenderOptions {
            sample_rate: SAMPLE_RATE,
            hz: f64::from(o.tone_hz) * 2f64.powf(semitones / 12.0),
            level: f64::from(o.volume.min(100)) / 100.0,
            ..RenderOptions::default()
        };
        let pcm = textweaver_cw::render(&s, &render);
        (s, pcm)
    }

    /// Ends the utterance whose audio is all in.
    fn close_open(&mut self) {
        if let Some(token) = self.open.take() {
            self.playback.on_end(token, EndStatus::Done);
        }
    }

    /// Emits what the playback clock reached, and records the characters
    /// that started to sound.
    fn emit(&mut self, sink: &mut dyn EventSink) {
        let mut tap = Tap {
            inner: sink,
            seen: Vec::new(),
        };
        self.playback.emit(&mut tap);
        let consumed = self.playback.feed().consumed();
        for (id, event) in tap.seen {
            match event {
                RawEvent::Word { byte_range, .. } => {
                    if let Some(q) = self.queued.iter_mut().find(|q| q.id == id)
                        && let Some((_, at)) = q.words.iter().find(|(r, _)| *r == byte_range)
                    {
                        q.anchor = Some((consumed, *at));
                    }
                }
                RawEvent::Finished => {
                    if let Some(i) = self.queued.iter().position(|q| q.id == id)
                        && let Some(mut q) = self.queued.remove(i)
                    {
                        record(&mut q, u64::MAX);
                        sending().skipped(q.skipped);
                    }
                    if self.queued.is_empty() {
                        sending().quiet();
                    }
                }
                RawEvent::Cancelled => self.queued.retain(|q| q.id != id),
                _ => {}
            }
        }
        if let Some(q) = self.queued.front_mut()
            && let Some((at, sample)) = q.anchor
        {
            let pos = sample + consumed.saturating_sub(at);
            record(q, pos);
        }
    }
}

/// Records `q`'s characters that start at or before sample `pos`.
fn record(q: &mut Queued, pos: u64) {
    while let Some((start, label, word)) = q.chars.get(q.next) {
        if *start > pos {
            break;
        }
        let new_word = q.last_word.is_some_and(|w| w != *word) || q.last_word.is_none();
        sending().sent(label, new_word);
        q.last_word = Some(*word);
        q.next += 1;
    }
}

impl SpeechBackend for CwBackend {
    fn id(&self) -> BackendId {
        BACKEND_ID
    }

    fn capabilities(&self) -> Caps {
        let mut caps = CAPS;
        if self.playback.output() == AudioOutput::Device {
            caps |= Caps::TONES;
        }
        caps
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        Ok(vec![Voice {
            id: "tone".into(),
            name: "Morse tone".into(),
            languages: Vec::new(),
            gender: None,
            tags: vec!["CW".into()],
        }])
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        self.playback.set_gain(params.volume.fraction());
        self.params = params.clone();
        Ok(())
    }

    fn effective_wpm(&self) -> u16 {
        options().effective_wpm
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        self.close_open();
        let (s, pcm) = self.render(&utterance.text);
        let token = self.playback.next_token();
        self.playback.enqueue(utterance.id, token, 0, ());
        if let Err(e) = self.playback.ensure_player() {
            self.playback.on_error(token, e.to_string());
            self.playback.on_end(token, EndStatus::Failed);
            self.playback.emit(sink);
            return Ok(());
        }
        let words: Vec<(Range<u32>, u64)> = s
            .words
            .iter()
            .map(|w| (w.byte_range.clone(), to_samples(w.start_ms, SAMPLE_RATE)))
            .collect();
        for (range, at) in &words {
            let range = range.clone();
            self.playback.on_word(token, *at, move |_| Some(range));
        }
        self.playback.on_audio(token, &pcm);
        self.open = Some(token);
        self.queued.push_back(Queued {
            id: utterance.id,
            chars: s
                .chars
                .iter()
                .map(|c| {
                    (
                        to_samples(c.start_ms, SAMPLE_RATE),
                        c.symbol.label(),
                        c.word,
                    )
                })
                .collect(),
            words,
            skipped: s.skipped,
            next: 0,
            last_word: None,
            anchor: None,
        });
        self.emit(sink);
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.close_open();
        self.emit(sink);
    }

    fn silence_after(&mut self, id: UtteranceId, ms: u32) {
        self.playback.silence_after(id, ms);
    }

    fn stop(&mut self) {
        self.open = None;
        self.playback.stop();
        self.queued.clear();
        sending().quiet();
    }

    fn pause(&mut self) -> Result<(), SpeechError> {
        self.playback.pause();
        Ok(())
    }

    fn resume(&mut self) -> Result<(), SpeechError> {
        self.playback.resume();
        Ok(())
    }

    fn tone(&mut self, hz: f32, ms: u32) {
        self.playback.tone(hz, ms);
    }

    fn synthesize_to_file(&mut self, text: &str, path: &Path) -> Result<(), SpeechError> {
        let u = Utterance::literal(text, textweaver_core::CharPos(0));
        self.synthesize_utterance(&u, path).map(|_| ())
    }

    fn synthesize_utterance(
        &mut self,
        utterance: &Utterance,
        path: &Path,
    ) -> Result<FileSynthesis, SpeechError> {
        let (s, pcm) = self.render(&utterance.text);
        let pcm = wav::apply_gain(pcm, self.playback.gain());
        wav::write(path, &pcm, SAMPLE_RATE).map_err(|e| SpeechError::Io(e.to_string()))?;
        Ok(FileSynthesis {
            words: s
                .words
                .iter()
                .map(|w| {
                    let sample = to_samples(w.start_ms, SAMPLE_RATE);
                    WordTiming {
                        byte_range: w.byte_range.clone(),
                        audio_ms: u32::try_from(sample * 1000 / u64::from(SAMPLE_RATE))
                            .unwrap_or(u32::MAX),
                    }
                })
                .collect(),
        })
    }
}

/// The sidetone heard while keying Morse code: a tone at the
/// `[speech.cw]` sidetone pitch and the CW level, on the audio device,
/// following the key ([`Sidetone::set`]). It plays apart from speech, with
/// the output's small buffer as its only delay. Tests never start one: it
/// has no silent output.
pub struct Sidetone {
    key: Arc<AtomicBool>,
    /// The pitch in hertz and the level in percent, packed by [`pack`],
    /// read by the generator at each sample.
    tone: Arc<AtomicU32>,
    _player: textweaver_enginehost::Player,
}

/// `hz` and `volume` in one word, for the generator to read at once.
fn pack(hz: u16, volume: u8) -> u32 {
    (u32::from(hz) << 8) | u32::from(volume.min(100))
}

/// The pitch in hertz and the level from 0 to 1 that [`pack`] packed.
fn unpack(tone: u32) -> (f64, f64) {
    (f64::from(tone >> 8), f64::from(tone & 0xff) / 100.0)
}

impl std::fmt::Debug for Sidetone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sidetone").finish_non_exhaustive()
    }
}

impl Sidetone {
    /// Opens the audio device for a sidetone of `hz` hertz at `volume`
    /// percent of full scale.
    pub fn start(hz: u16, volume: u8) -> Result<Self, SpeechError> {
        let key = Arc::new(AtomicBool::new(false));
        let packed = pack(hz, volume);
        let tone = Arc::new(AtomicU32::new(packed));
        let (pitch, level) = unpack(packed);
        let mut osc = textweaver_cw::Sidetone::new(
            SAMPLE_RATE,
            pitch,
            level,
            RenderOptions::default().edge_ms,
        );
        let (down, wanted) = (Arc::clone(&key), Arc::clone(&tone));
        let mut current = packed;
        let player =
            textweaver_enginehost::Player::start_generator(BACKEND_ID, SAMPLE_RATE, move || {
                let now = wanted.load(Ordering::Relaxed);
                if now != current {
                    current = now;
                    let (pitch, level) = unpack(now);
                    osc.retune(SAMPLE_RATE, pitch, level);
                }
                osc.next(down.load(Ordering::Relaxed))
            })?;
        Ok(Sidetone {
            key,
            tone,
            _player: player,
        })
    }

    /// The key went down (true) or up: the tone rises or falls.
    pub fn set(&self, down: bool) {
        self.key.store(down, Ordering::Relaxed);
    }

    /// The pitch or level changed: heard from the next sample, without a
    /// click, even with the key down.
    pub fn set_tone(&self, hz: u16, volume: u8) {
        self.tone.store(pack(hz, volume), Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use textweaver_core::{CharPos, Rate};

    use super::*;

    #[derive(Default)]
    struct Rec(Vec<(UtteranceId, RawEvent)>);

    impl EventSink for Rec {
        fn emit(&mut self, id: UtteranceId, event: RawEvent) {
            self.0.push((id, event));
        }
        fn is_current(&self, _id: UtteranceId) -> bool {
            true
        }
    }

    fn id(chunk: u32) -> UtteranceId {
        UtteranceId {
            generation: 1,
            chunk,
        }
    }

    /// Polls until `id` finishes (the silent output plays fast).
    fn run(b: &mut CwBackend, rec: &mut Rec, last: UtteranceId) {
        let deadline = Instant::now() + Duration::from_secs(30);
        while !rec.0.contains(&(last, RawEvent::Finished)) {
            assert!(Instant::now() < deadline, "no Finished: {:?}", rec.0);
            b.poll(rec);
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn the_sidetone_pitch_and_level_pack_into_one_word() {
        assert_eq!(unpack(pack(600, 50)), (600.0, 0.5));
        assert_eq!(unpack(pack(1500, 100)), (1500.0, 1.0));
        assert_eq!(unpack(pack(200, 0)), (200.0, 0.0));
        // A level above 100 percent is full scale.
        assert_eq!(unpack(pack(800, 255)), (800.0, 1.0));
    }

    #[test]
    fn the_entry_is_opt_in_and_never_a_fallback() {
        let d = backend_description();
        assert!(d.opt_in);
        assert!(d.caps.contains(Caps::RAW_TEXT | Caps::PLAYBACK_EVENTS));
        let order = textweaver_speech::backends::start_order_from(&[d], None);
        assert!(order.iter().all(|b| b.id != BACKEND_ID), "{order:?}");
    }

    #[test]
    fn words_sound_in_order_with_exact_times_and_the_status_line_follows() {
        let mut b = CwBackend::new(AudioOutput::Null { speed: 50.0 });
        let mut rec = Rec::default();
        b.set_params(&VoiceParams {
            rate: Rate::default(),
            ..VoiceParams::default()
        })
        .unwrap();
        sending().clear();
        let u = Utterance {
            id: id(0),
            ..Utterance::literal("CQ de <BT> #", CharPos(0))
        };
        b.speak(&u, &mut rec).unwrap();
        run(&mut b, &mut rec, id(0));
        let words: Vec<(Range<u32>, Option<u32>)> = rec
            .0
            .iter()
            .filter_map(|(_, e)| match e {
                RawEvent::Word {
                    byte_range,
                    audio_ms,
                } => Some((byte_range.clone(), *audio_ms)),
                _ => None,
            })
            .collect();
        let s = schedule("CQ de <BT> #", &options().schedule_options());
        let want: Vec<(Range<u32>, Option<u32>)> = s
            .words
            .iter()
            .map(|w| {
                let at = to_samples(w.start_ms, SAMPLE_RATE);
                (
                    w.byte_range.clone(),
                    Some(u32::try_from(at * 1000 / u64::from(SAMPLE_RATE)).unwrap()),
                )
            })
            .collect();
        assert_eq!(words, want);
        assert_eq!(rec.0.first(), Some(&(id(0), RawEvent::Started)));
        let snap = sending().snapshot();
        assert_eq!(snap.tail, "CQ DE <BT>");
        assert_eq!(snap.skipped, 1);
        assert!(snap.current.is_none());
    }

    #[test]
    fn export_writes_cw_with_word_times() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cw.wav");
        let mut b = CwBackend::new(AudioOutput::Null { speed: 50.0 });
        let u = Utterance::literal("Hi there", CharPos(0));
        let out = b.synthesize_utterance(&u, &path).unwrap();
        assert_eq!(out.words.len(), 2);
        assert_eq!(out.words[0].byte_range, 0..2);
        assert_eq!(out.words[0].audio_ms, 0);
        assert!(out.words[1].audio_ms > 0);
        let bytes = std::fs::read(&path).unwrap();
        let (rate, pcm) = wav::read_wav(&bytes).unwrap();
        assert_eq!(rate, SAMPLE_RATE);
        assert!(pcm.iter().any(|s| *s != 0));
    }

    #[test]
    fn stop_cancels_and_goes_quiet() {
        let mut b = CwBackend::new(AudioOutput::Null { speed: 0.0 });
        let mut rec = Rec::default();
        b.speak(&Utterance::literal("PARIS", CharPos(0)), &mut rec)
            .unwrap();
        b.stop();
        b.poll(&mut rec);
        assert!(rec.0.iter().any(|(_, e)| *e == RawEvent::Cancelled));
        assert!(b.queued.is_empty());
    }
}
