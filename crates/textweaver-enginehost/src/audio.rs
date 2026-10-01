//! Playback: a sample queue shared between a backend and an output.
//!
//! The backend pushes a host's PCM into a [`Feed`]; an output pulls from it
//! at the sample rate and counts what it consumed. That count is the
//! playback clock: word events fire when it passes a word's sample.
//! Pausing makes the output play silence without consuming, so the clock
//! stops with the audio (native pause and resume); stopping discards the
//! queue.
//!
//! Two outputs exist: an audio device through rodio (feature `playback`),
//! and [`AudioOutput::Null`], which consumes samples on a timer without
//! making a sound (tests; never play audio aloud in tests).
//!
//! The device is the one chosen with [`set_output_device`] (`[speech]
//! output_device`, by its stable id from [`output_devices`]) when it is
//! connected, else the system's default. When the device goes away (a
//! headset unplugged) or its stream must be rebuilt, the player says so
//! ([`Player::lost`]) and the playback client opens it again, which
//! finds the current default; audio waiting in the feed is kept.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use textweaver_speech::{BackendId, SpeechError};

/// The output device chosen by its stable id, and a count of changes so
/// open players notice one.
static OUTPUT_DEVICE: Mutex<Option<String>> = Mutex::new(None);
static OUTPUT_DEVICE_CHANGES: AtomicU64 = AtomicU64::new(0);

/// Chooses the output device by its stable id ([`OutputDevice::id`]);
/// `None` (or an empty id) uses the system's default. It applies to every
/// device output in the process: players open on it from now on, and open
/// ones reopen on it ([`Player::choice_changed`]). A chosen device that is
/// not connected falls back to the default, with a line in the log.
pub fn set_output_device(id: Option<String>) {
    let id = id.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    let mut current = OUTPUT_DEVICE.lock().unwrap_or_else(|e| e.into_inner());
    if *current != id {
        *current = id;
        OUTPUT_DEVICE_CHANGES.fetch_add(1, Ordering::AcqRel);
    }
}

/// The output device chosen with [`set_output_device`], if any.
pub fn output_device() -> Option<String> {
    OUTPUT_DEVICE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// An audio output device ([`output_devices`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputDevice {
    /// Its stable id, for `[speech] output_device`: the audio system's own
    /// name for it, which stays the same across restarts and reconnects
    /// (on Windows, `wasapi:` and the endpoint id).
    pub id: String,
    /// Its name, as the system shows it ("Speakers (Realtek Audio)").
    pub name: String,
    /// The system's default output now.
    pub default: bool,
}

/// The output devices connected now, the default first. Empty when the
/// build has no audio output (no `playback` feature); an error says why
/// the list could not be read.
pub fn output_devices() -> Result<Vec<OutputDevice>, String> {
    #[cfg(feature = "playback")]
    {
        device::list()
    }
    #[cfg(not(feature = "playback"))]
    {
        Ok(Vec::new())
    }
}

/// Where audio goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AudioOutput {
    /// An audio device: the one [`set_output_device`] chose when it is
    /// connected, else the default (requires the `playback` feature).
    Device,
    /// Nowhere: samples are consumed at `speed` times real time, silently.
    /// A speed of 0 (or less) consumes nothing: a device that has stopped
    /// taking samples, for tests.
    Null {
        /// Playback speed factor (1.0 is real time).
        speed: f32,
    },
}

impl Default for AudioOutput {
    fn default() -> Self {
        if cfg!(feature = "playback") {
            AudioOutput::Device
        } else {
            AudioOutput::Null { speed: 1.0 }
        }
    }
}

#[derive(Debug, Default)]
struct FeedState {
    queue: VecDeque<i16>,
    pushed: u64,
    consumed: u64,
    paused: bool,
}

/// The shared sample queue and playback clock.
#[derive(Debug)]
pub struct Feed {
    state: Mutex<FeedState>,
    gain: AtomicU32,
}

impl Default for Feed {
    fn default() -> Self {
        Feed {
            state: Mutex::default(),
            gain: AtomicU32::new(1.0f32.to_bits()),
        }
    }
}

impl Feed {
    fn lock(&self) -> MutexGuard<'_, FeedState> {
        // A panic while holding the lock cannot leave the queue invalid.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Queues samples; returns the clock position of the first one.
    pub fn push(&self, samples: &[i16]) -> u64 {
        let mut s = self.lock();
        let at = s.pushed;
        s.queue.extend(samples.iter().copied());
        s.pushed += samples.len() as u64;
        at
    }

    /// Fills `out` with up to `out.len()` samples as floats (gain applied)
    /// and returns how many were real samples; the rest of `out` is
    /// silence. Paused: all silence, nothing consumed.
    pub fn pull(&self, out: &mut [f32]) -> usize {
        let gain = self.gain();
        let mut s = self.lock();
        let n = if s.paused {
            0
        } else {
            out.len().min(s.queue.len())
        };
        for (o, v) in out.iter_mut().zip(s.queue.drain(..n)) {
            *o = f32::from(v) / 32768.0 * gain;
        }
        s.consumed += n as u64;
        drop(s);
        for o in &mut out[n..] {
            *o = 0.0;
        }
        n
    }

    /// Consumes up to `n` samples without producing output (the null
    /// output's clock). Paused: nothing.
    pub fn skip(&self, n: usize) -> usize {
        let mut s = self.lock();
        if s.paused {
            return 0;
        }
        let n = n.min(s.queue.len());
        s.queue.drain(..n);
        s.consumed += n as u64;
        n
    }

    /// Discards everything queued; the clock jumps to the end.
    pub fn clear(&self) {
        let mut s = self.lock();
        s.queue.clear();
        s.consumed = s.pushed;
    }

    /// Samples pushed so far (where the next push starts).
    pub fn pushed(&self) -> u64 {
        self.lock().pushed
    }

    /// Samples played so far (the playback clock).
    pub fn consumed(&self) -> u64 {
        self.lock().consumed
    }

    /// Pauses or resumes the clock.
    pub fn set_paused(&self, paused: bool) {
        self.lock().paused = paused;
    }

    /// Whether the clock is paused.
    pub fn is_paused(&self) -> bool {
        self.lock().paused
    }

    /// Sets the linear output gain (volume), applied immediately.
    pub fn set_gain(&self, gain: f32) {
        self.gain.store(gain.to_bits(), Ordering::Relaxed);
    }

    /// The current gain.
    pub fn gain(&self) -> f32 {
        f32::from_bits(self.gain.load(Ordering::Relaxed))
    }
}

/// The audio device, opened on its own thread (Phase 2: opening a device
/// can take a moment, and a Bluetooth headset longer; the speech thread
/// never waits for it). The thread keeps the device open until the player
/// is dropped.
#[cfg(feature = "playback")]
struct Device {
    /// The device's mixer once it opened, or why it could not.
    opened: Mutex<Option<Result<rodio::mixer::Mixer, String>>>,
    /// Receives the outcome of opening.
    answer: Mutex<std::sync::mpsc::Receiver<Result<rodio::mixer::Mixer, String>>>,
    /// Dropped with the player: the device thread closes the device.
    _close: std::sync::mpsc::Sender<()>,
}

#[cfg(feature = "playback")]
impl Device {
    /// The outcome of opening, if it has arrived.
    fn state(&self) -> Option<Result<rodio::mixer::Mixer, String>> {
        let mut opened = self.opened.lock().unwrap_or_else(|e| e.into_inner());
        if opened.is_none()
            && let Ok(r) = self
                .answer
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .try_recv()
        {
            *opened = Some(r);
        }
        opened.clone()
    }
}

/// Why an open output was lost, set from the audio system's thread.
type Lost = Arc<Mutex<Option<String>>>;

/// A running output. Dropping it stops playback.
pub struct Player {
    // Field order: the device (if any) closes before the timer thread stops.
    #[cfg(feature = "playback")]
    device: Option<Device>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// Set when the device went away or its stream must be rebuilt.
    lost: Lost,
    /// [`OUTPUT_DEVICE_CHANGES`] when a device output opened; `None` for
    /// the silent output, which has no device to choose.
    choice: Option<u64>,
}

impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player").finish_non_exhaustive()
    }
}

impl Player {
    /// Starts `output` pulling from `feed` at `sample_rate` Hz. Errors name
    /// the backend `"enginehost"`; backends use [`Player::start_for`].
    pub fn start(
        output: AudioOutput,
        feed: Arc<Feed>,
        sample_rate: u32,
    ) -> Result<Self, SpeechError> {
        Self::start_for("enginehost", output, feed, sample_rate)
    }

    /// Starts `output` for `backend` (named in errors and thread names).
    pub fn start_for(
        backend: BackendId,
        output: AudioOutput,
        feed: Arc<Feed>,
        sample_rate: u32,
    ) -> Result<Self, SpeechError> {
        match output {
            AudioOutput::Null { speed } => Ok(Self::null(backend, feed, sample_rate, speed)),
            AudioOutput::Device => Self::device(backend, feed, sample_rate),
        }
    }

    fn null(backend: BackendId, feed: Arc<Feed>, sample_rate: u32, speed: f32) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name(format!("{backend}-null-output"))
                .spawn(move || {
                    let rate = if speed > 0.0 {
                        f64::from(sample_rate) * f64::from(speed.max(0.01))
                    } else {
                        0.0
                    };
                    let mut last = Instant::now();
                    let mut owed = 0.0f64;
                    while !stop.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(2));
                        let now = Instant::now();
                        owed += now.duration_since(last).as_secs_f64() * rate;
                        last = now;
                        let whole = owed.floor();
                        owed -= whole;
                        // Unused capacity is not banked: an idle output
                        // must not burst through the next utterance.
                        let _ = feed.skip(whole as usize);
                    }
                })
                .ok()
        };
        Player {
            #[cfg(feature = "playback")]
            device: None,
            stop,
            thread,
            lost: Lost::default(),
            choice: None,
        }
    }

    /// Opens the chosen device ([`set_output_device`]), or the default,
    /// on its own thread and returns at once; audio pushed meanwhile waits
    /// in the feed. A device that cannot be opened is reported by
    /// [`failure`](Self::failure), one that goes away later by
    /// [`lost`](Self::lost).
    #[cfg(feature = "playback")]
    fn device(backend: BackendId, feed: Arc<Feed>, sample_rate: u32) -> Result<Self, SpeechError> {
        let rate = std::num::NonZero::new(sample_rate)
            .ok_or_else(|| SpeechError::Engine("sample rate 0".into()))?;
        let (answer_tx, answer) = std::sync::mpsc::channel();
        let (close, closed) = std::sync::mpsc::channel::<()>();
        let lost = Lost::default();
        let choice = OUTPUT_DEVICE_CHANGES.load(Ordering::Acquire);
        let wanted = output_device();
        let on_error = device::on_error(backend, Arc::clone(&lost));
        std::thread::Builder::new()
            .name(format!("{backend}-audio-device"))
            .spawn(move || {
                let mut sink = match device::open(wanted.as_deref(), on_error) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = answer_tx.send(Err(format!("no audio output: {e}")));
                        return;
                    }
                };
                sink.log_on_drop(false);
                sink.mixer().add(FeedSource {
                    feed,
                    rate,
                    buf: vec![0.0; 64],
                    pos: 64,
                });
                let _ = answer_tx.send(Ok(sink.mixer().clone()));
                // Open until the player is dropped (its sender closes).
                let _ = closed.recv();
                drop(sink);
            })
            .map_err(|e| SpeechError::Io(format!("cannot start the audio thread: {e}")))?;
        Ok(Player {
            device: Some(Device {
                opened: Mutex::new(None),
                answer: Mutex::new(answer),
                _close: close,
            }),
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
            lost,
            choice: Some(choice),
        })
    }

    #[cfg(not(feature = "playback"))]
    fn device(
        backend: BackendId,
        _feed: Arc<Feed>,
        _sample_rate: u32,
    ) -> Result<Self, SpeechError> {
        Err(SpeechError::Unavailable(
            backend,
            "built without the playback feature".into(),
        ))
    }

    /// Plays a sine tone of `hz` for `ms` milliseconds at `gain`, mixed
    /// over speech (device output only, once it is open).
    pub fn tone(&self, hz: f32, ms: u32, gain: f32) {
        #[cfg(feature = "playback")]
        if let Some(Some(Ok(mixer))) = self.device.as_ref().map(Device::state) {
            use rodio::Source;
            let tone = rodio::source::SineWave::new(hz)
                .take_duration(Duration::from_millis(u64::from(ms)))
                .amplify(0.25 * gain);
            mixer.add(tone);
        }
        #[cfg(not(feature = "playback"))]
        let _ = (hz, ms, gain);
    }

    /// True once the output is taking samples (the device opened; the
    /// silent output at once).
    pub fn is_open(&self) -> bool {
        #[cfg(feature = "playback")]
        if let Some(d) = &self.device {
            return matches!(d.state(), Some(Ok(_)));
        }
        true
    }

    /// Why the device could not be opened, once that is known.
    pub fn failure(&self) -> Option<String> {
        #[cfg(feature = "playback")]
        if let Some(d) = &self.device
            && let Some(Err(e)) = d.state()
        {
            return Some(e);
        }
        None
    }

    /// Why the open output stopped working, once the audio system said
    /// so: its device went away (unplugged, switched off) or its stream
    /// must be rebuilt. Opening it again finds the current device.
    pub fn lost(&self) -> Option<String> {
        self.lost.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Marks the output lost, as the audio system's error callback does
    /// (tests).
    #[cfg(test)]
    pub(crate) fn mark_lost(&self, why: &str) {
        mark_lost(&self.lost, why);
    }

    /// True when the output device was chosen again
    /// ([`set_output_device`]) after this device output opened, so it
    /// should open again on the new one. Always false for the silent
    /// output.
    pub fn choice_changed(&self) -> bool {
        self.choice
            .is_some_and(|c| c != OUTPUT_DEVICE_CHANGES.load(Ordering::Acquire))
    }
}

/// Records why an output was lost; the first reason stays.
#[cfg_attr(not(any(test, feature = "playback")), allow(dead_code))]
fn mark_lost(lost: &Lost, why: &str) {
    let mut slot = lost.lock().unwrap_or_else(|e| e.into_inner());
    if slot.is_none() {
        *slot = Some(why.to_owned());
    }
}

/// Opening and listing devices through rodio's cpal.
#[cfg(feature = "playback")]
mod device {
    use rodio::cpal::StreamError;
    use rodio::cpal::traits::{DeviceTrait, HostTrait};
    use rodio::{DeviceSinkBuilder, MixerDeviceSink};

    use super::{Lost, OutputDevice, mark_lost};

    /// The error callback for an output's stream: a device that went away
    /// or a stream that must be rebuilt marks the output lost; other
    /// errors (an underrun) only go to the log. Never to stderr, which is
    /// the terminal reader's screen.
    pub(super) fn on_error(
        backend: &'static str,
        lost: Lost,
    ) -> impl FnMut(StreamError) + Send + Clone + 'static {
        move |e: StreamError| match e {
            StreamError::DeviceNotAvailable => {
                mark_lost(&lost, "the audio output device went away");
            }
            StreamError::StreamInvalidated => {
                mark_lost(&lost, "the audio output must be opened again");
            }
            other => log::debug!("{backend}: audio stream: {other}"),
        }
    }

    /// The connected output device with stable id `id`.
    fn find(id: &str) -> Option<rodio::cpal::Device> {
        let id: rodio::cpal::DeviceId = id.parse().ok()?;
        rodio::cpal::default_host().device_by_id(&id)
    }

    /// Opens the device with id `wanted` when it is connected, else the
    /// default, else the first other device that opens.
    pub(super) fn open(
        wanted: Option<&str>,
        on_error: impl FnMut(StreamError) + Send + Clone + 'static,
    ) -> Result<MixerDeviceSink, String> {
        if let Some(id) = wanted {
            match find(id) {
                Some(d) => {
                    match DeviceSinkBuilder::from_device(d)
                        .and_then(|b| b.with_error_callback(on_error.clone()).open_stream())
                    {
                        Ok(s) => return Ok(s),
                        Err(e) => log::warn!(
                            "cannot open the chosen audio output {id} ({e}); using the default"
                        ),
                    }
                }
                None => {
                    log::warn!("the chosen audio output {id} is not connected; using the default")
                }
            }
        }
        let host = rodio::cpal::default_host();
        let first = match host.default_output_device() {
            Some(d) => DeviceSinkBuilder::from_device(d)
                .and_then(|b| {
                    b.with_error_callback(on_error.clone())
                        .open_sink_or_fallback()
                })
                .map_err(|e| e.to_string()),
            None => Err("there is no default output device".to_owned()),
        };
        first.or_else(|first| {
            let devices = host.output_devices().map_err(|_| first.clone())?;
            devices
                .filter(|d| {
                    d.description()
                        .is_ok_and(|desc| desc.driver().is_none_or(|driver| driver != "null"))
                })
                .find_map(|d| {
                    DeviceSinkBuilder::from_device(d)
                        .and_then(|b| {
                            b.with_error_callback(on_error.clone())
                                .open_sink_or_fallback()
                        })
                        .ok()
                })
                .ok_or(first)
        })
    }

    /// The connected output devices, the default first.
    pub(super) fn list() -> Result<Vec<OutputDevice>, String> {
        let host = rodio::cpal::default_host();
        let default = host
            .default_output_device()
            .and_then(|d| d.id().ok())
            .map(|id| id.to_string());
        let mut out: Vec<OutputDevice> = host
            .output_devices()
            .map_err(|e| format!("cannot list the audio outputs: {e}"))?
            .filter_map(|d| {
                let id = d.id().ok()?.to_string();
                let name = d
                    .description()
                    .map(|desc| desc.name().to_owned())
                    .unwrap_or_else(|_| id.clone());
                Some(OutputDevice {
                    default: default.as_deref() == Some(id.as_str()),
                    id,
                    name,
                })
            })
            .collect();
        out.sort_by_key(|d| !d.default);
        Ok(out)
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// An endless rodio source reading from the feed (silence when idle).
#[cfg(feature = "playback")]
struct FeedSource {
    feed: Arc<Feed>,
    rate: rodio::SampleRate,
    buf: Vec<f32>,
    pos: usize,
}

#[cfg(feature = "playback")]
impl Iterator for FeedSource {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.buf.len() {
            // 64-sample batches keep the clock within 6 ms at 11,025 Hz
            // (3 ms at 22,050 Hz) while taking the lock once per batch.
            self.feed.pull(&mut self.buf);
            self.pos = 0;
        }
        let v = self.buf[self.pos];
        self.pos += 1;
        Some(v)
    }
}

#[cfg(feature = "playback")]
impl rodio::Source for FeedSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        rodio::ChannelCount::MIN
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        self.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_counts_pushes_and_pulls() {
        let f = Feed::default();
        assert_eq!(f.push(&[1000; 10]), 0);
        assert_eq!(f.push(&[1000; 5]), 10);
        let mut out = [1.0f32; 8];
        assert_eq!(f.pull(&mut out), 8);
        assert!((out[0] - 1000.0 / 32768.0).abs() < 1e-6);
        assert_eq!(f.consumed(), 8);
        let mut out = [1.0f32; 16];
        assert_eq!(f.pull(&mut out), 7);
        assert_eq!(out[7], 0.0);
        assert_eq!((f.pushed(), f.consumed()), (15, 15));
    }

    #[test]
    fn pause_stops_the_clock_and_clear_jumps_it() {
        let f = Feed::default();
        f.push(&[1; 100]);
        f.set_paused(true);
        assert!(f.is_paused());
        assert_eq!(f.skip(50), 0);
        let mut out = [9.0f32; 4];
        assert_eq!(f.pull(&mut out), 0);
        assert_eq!(out, [0.0; 4]);
        f.set_paused(false);
        assert_eq!(f.skip(50), 50);
        f.clear();
        assert_eq!(f.consumed(), 100);
        assert_eq!(f.push(&[1; 3]), 100);
    }

    #[test]
    fn gain_scales_samples() {
        let f = Feed::default();
        f.set_gain(0.5);
        assert!((f.gain() - 0.5).abs() < f32::EPSILON);
        f.push(&[16384]);
        let mut out = [0.0f32; 1];
        f.pull(&mut out);
        assert!((out[0] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn null_output_consumes_in_real_time() {
        for rate in [8000u32, 22050] {
            let f = Arc::new(Feed::default());
            f.push(&vec![0; rate as usize]);
            let p = Player::start_for(
                "test",
                AudioOutput::Null { speed: 10.0 },
                Arc::clone(&f),
                rate,
            )
            .unwrap();
            let t0 = Instant::now();
            while f.consumed() < u64::from(rate) && t0.elapsed() < Duration::from_secs(5) {
                std::thread::sleep(Duration::from_millis(5));
            }
            drop(p);
            assert_eq!(f.consumed(), u64::from(rate));
            // One second of audio at 10x takes about 100 ms.
            assert!(
                t0.elapsed() >= Duration::from_millis(60),
                "{:?}",
                t0.elapsed()
            );
        }
    }

    #[test]
    fn choosing_the_output_device_is_noticed_only_by_device_outputs() {
        // The only test that touches the process-wide choice. The silent
        // output has no device, so other tests' players never reopen
        // because of it.
        let null =
            Player::start_for("t", AudioOutput::Null { speed: 1.0 }, Arc::default(), 8000).unwrap();
        let before = OUTPUT_DEVICE_CHANGES.load(Ordering::Acquire);
        set_output_device(Some("  wasapi:{0.0.0.00000000}.{1234}  ".into()));
        assert_eq!(
            output_device().as_deref(),
            Some("wasapi:{0.0.0.00000000}.{1234}"),
            "trimmed"
        );
        let after = OUTPUT_DEVICE_CHANGES.load(Ordering::Acquire);
        assert_eq!(after, before + 1);
        // The same choice again is no change.
        set_output_device(Some("wasapi:{0.0.0.00000000}.{1234}".into()));
        assert_eq!(OUTPUT_DEVICE_CHANGES.load(Ordering::Acquire), after);
        assert!(!null.choice_changed());
        // A player that opened before a change sees it.
        let device = Player {
            #[cfg(feature = "playback")]
            device: None,
            stop: Arc::default(),
            thread: None,
            lost: Lost::default(),
            choice: Some(after),
        };
        assert!(!device.choice_changed());
        // Empty means the default.
        set_output_device(Some(" ".into()));
        assert_eq!(output_device(), None);
        assert!(device.choice_changed());
        set_output_device(None);
        assert_eq!(output_device(), None);
    }

    #[test]
    fn a_lost_output_keeps_the_first_reason() {
        let p =
            Player::start_for("t", AudioOutput::Null { speed: 1.0 }, Arc::default(), 8000).unwrap();
        assert_eq!(p.lost(), None);
        p.mark_lost("the device went away");
        p.mark_lost("the stream must be rebuilt");
        assert_eq!(p.lost().as_deref(), Some("the device went away"));
    }

    #[cfg(not(feature = "playback"))]
    #[test]
    fn without_playback_the_device_is_unavailable() {
        let e = Player::start_for("x", AudioOutput::Device, Arc::default(), 8000).unwrap_err();
        assert!(matches!(e, SpeechError::Unavailable("x", _)));
        assert_eq!(AudioOutput::default(), AudioOutput::Null { speed: 1.0 });
        assert_eq!(output_devices(), Ok(Vec::new()));
    }
}
