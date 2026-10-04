//! Audio to dictate from: recorded samples and the capture trait a
//! microphone implements.
//!
//! Microphone capture itself needs an audio input library (`cpal`), which
//! is not in the workspace yet (ADR-0013); the app plugs a microphone in
//! through [`AudioCapture`] when it is. [`BufferCapture`] serves tests and
//! audio recorded elsewhere.

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::DictationError;

/// Whisper's native sample rate.
pub const WHISPER_SAMPLE_RATE: u32 = 16_000;

/// Mono 16-bit PCM audio.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pcm {
    /// Samples per second.
    pub sample_rate: u32,
    /// The samples.
    pub samples: Vec<i16>,
}

impl Pcm {
    /// Audio of `samples` at `sample_rate`.
    pub fn new(sample_rate: u32, samples: Vec<i16>) -> Self {
        Pcm {
            sample_rate,
            samples,
        }
    }

    /// How long the audio lasts.
    pub fn duration(&self) -> Duration {
        if self.sample_rate == 0 {
            return Duration::ZERO;
        }
        let n = u64::try_from(self.samples.len()).unwrap_or(u64::MAX);
        Duration::from_millis(n.saturating_mul(1000) / u64::from(self.sample_rate))
    }

    /// True when there is no audio.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// The audio as a WAV file (RIFF, 16-bit PCM, mono).
    pub fn to_wav(&self) -> Vec<u8> {
        let data_len = u32::try_from(self.samples.len().saturating_mul(2)).unwrap_or(u32::MAX);
        let mut out = Vec::with_capacity(44 + self.samples.len() * 2);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&data_len.saturating_add(36).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&1u16.to_le_bytes()); // mono
        out.extend_from_slice(&self.sample_rate.to_le_bytes());
        out.extend_from_slice(&self.sample_rate.saturating_mul(2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes()); // block align
        out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for s in &self.samples {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out
    }

    /// Writes the audio to `path` as a WAV file.
    pub fn write_wav(&self, path: &Path) -> Result<(), DictationError> {
        let io = |source| DictationError::Io {
            path: path.to_owned(),
            source,
        };
        let mut f = std::fs::File::create(path).map_err(io)?;
        f.write_all(&self.to_wav()).map_err(io)?;
        f.flush().map_err(io)
    }
}

/// A source of recorded speech, such as a microphone: recording runs
/// between `start` and `stop`, with no fixed length (star's
/// `StreamRecorder`: "press Stop when you're done").
pub trait AudioCapture: Send {
    /// Starts recording.
    fn start(&mut self) -> Result<(), DictationError>;

    /// Stops recording and returns what was captured.
    fn stop(&mut self) -> Result<Pcm, DictationError>;

    /// Stops recording and discards it.
    fn cancel(&mut self);

    /// How long recording has run, for a live timer.
    fn elapsed(&self) -> Duration;

    /// Live audio, for dictation that transcribes while the speaker talks.
    /// Called before `start`: the capture then adds its recording to the
    /// returned [`LiveAudio`] as it records, at 16 kHz, and ends it when
    /// recording stops or is cancelled. `None` (the default) when the
    /// capture delivers its recording only at `stop`.
    fn live(&mut self) -> Option<LiveAudio> {
        None
    }
}

/// Audio shared while it is recorded: 16 kHz mono samples in
/// `-1.0..=1.0`, added by a capture and taken by the transcriber, until
/// the capture ends it. Clones share the same audio.
#[derive(Clone, Default)]
pub struct LiveAudio {
    inner: Arc<(Mutex<LiveBuffer>, Condvar)>,
}

#[derive(Default)]
struct LiveBuffer {
    /// Added and not yet taken.
    samples: Vec<f32>,
    /// Added in all.
    total: usize,
    ended: bool,
}

impl std::fmt::Debug for LiveAudio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let b = self.lock();
        f.debug_struct("LiveAudio")
            .field("total", &b.total)
            .field("waiting", &b.samples.len())
            .field("ended", &b.ended)
            .finish()
    }
}

impl LiveAudio {
    /// Empty live audio, not ended.
    pub fn new() -> Self {
        LiveAudio::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, LiveBuffer> {
        self.inner.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Adds recorded samples; ignored once the audio has ended.
    pub fn push(&self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        let mut b = self.lock();
        if b.ended {
            return;
        }
        b.samples.extend_from_slice(samples);
        b.total += samples.len();
        drop(b);
        self.inner.1.notify_all();
    }

    /// Ends the audio: nothing more will be added. Ending twice is
    /// harmless.
    pub fn end(&self) {
        self.lock().ended = true;
        self.inner.1.notify_all();
    }

    /// Samples added so far.
    pub fn total(&self) -> usize {
        self.lock().total
    }

    /// Takes the samples added since the last take, waiting up to `wait`
    /// when there are none yet. The flag is true when the audio has ended
    /// and everything has been taken.
    pub fn take(&self, wait: Duration) -> (Vec<f32>, bool) {
        let mut b = self.lock();
        if b.samples.is_empty() && !b.ended && !wait.is_zero() {
            b = self
                .inner
                .1
                .wait_timeout_while(b, wait, |b| b.samples.is_empty() && !b.ended)
                .map(|(b, _)| b)
                .unwrap_or_else(|e| e.into_inner().0);
        }
        let samples = std::mem::take(&mut b.samples);
        (samples, b.ended)
    }
}

/// Floats in `-1.0..=1.0` as 16-bit samples.
fn to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        // In range after the clamp.
        .map(|&s| (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16)
        .collect()
}

/// 16-bit samples as floats in `-1.0..=1.0`.
fn to_f32(samples: &[i16]) -> Vec<f32> {
    samples.iter().map(|&s| f32::from(s) / 32768.0).collect()
}

/// Recorded 16 kHz audio played into live dictation at the pace it was
/// spoken, as a microphone would deliver it, in 20 ms pieces. Nothing is
/// played aloud. The recording ends by itself when the audio runs out;
/// `stop` ends it sooner. For `tw dictate --live --file` and tests.
pub struct PacedCapture {
    audio: Arc<Vec<f32>>,
    live: Option<LiveAudio>,
    stop: Arc<AtomicBool>,
    played: Arc<AtomicUsize>,
    thread: Option<JoinHandle<()>>,
    started: Option<Instant>,
}

impl std::fmt::Debug for PacedCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PacedCapture")
            .field("samples", &self.audio.len())
            .field("recording", &self.thread.is_some())
            .finish_non_exhaustive()
    }
}

/// The piece a paced capture adds at a time: 20 ms at 16 kHz.
const PACED_PIECE: usize = 320;

impl PacedCapture {
    /// A capture that plays `samples` (16 kHz, `-1.0..=1.0`).
    pub fn new(samples: Vec<f32>) -> Self {
        PacedCapture {
            audio: Arc::new(samples),
            live: None,
            stop: Arc::new(AtomicBool::new(false)),
            played: Arc::new(AtomicUsize::new(0)),
            thread: None,
            started: None,
        }
    }

    fn finish(&mut self) -> usize {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        if let Some(live) = &self.live {
            live.end();
        }
        self.started = None;
        self.played.load(Ordering::SeqCst)
    }
}

impl AudioCapture for PacedCapture {
    fn start(&mut self) -> Result<(), DictationError> {
        if self.thread.is_some() {
            return Err(DictationError::Busy);
        }
        self.stop = Arc::new(AtomicBool::new(false));
        self.played = Arc::new(AtomicUsize::new(0));
        let (audio, live) = (Arc::clone(&self.audio), self.live.clone());
        let (stop, played) = (Arc::clone(&self.stop), Arc::clone(&self.played));
        let rate = f64::from(WHISPER_SAMPLE_RATE);
        let thread = std::thread::Builder::new()
            .name("textweaver-paced-audio".into())
            .spawn(move || {
                let t0 = Instant::now();
                for (k, piece) in audio.chunks(PACED_PIECE).enumerate() {
                    // A piece is due when it has been "spoken".
                    let due = t0
                        + Duration::from_secs_f64(((k * PACED_PIECE + piece.len()) as f64) / rate);
                    while !stop.load(Ordering::SeqCst) {
                        let now = Instant::now();
                        if now >= due {
                            break;
                        }
                        std::thread::sleep((due - now).min(Duration::from_millis(20)));
                    }
                    if stop.load(Ordering::SeqCst) {
                        return;
                    }
                    if let Some(live) = &live {
                        live.push(piece);
                    }
                    played.fetch_add(piece.len(), Ordering::SeqCst);
                }
                if let Some(live) = &live {
                    live.end();
                }
            })
            .map_err(|e| DictationError::Capture(e.to_string()))?;
        self.thread = Some(thread);
        self.started = Some(Instant::now());
        Ok(())
    }

    fn stop(&mut self) -> Result<Pcm, DictationError> {
        if self.thread.is_none() {
            return Err(DictationError::NotRecording);
        }
        let n = self.finish();
        Ok(Pcm::new(WHISPER_SAMPLE_RATE, to_i16(&self.audio[..n])))
    }

    fn cancel(&mut self) {
        let _ = self.finish();
    }

    fn elapsed(&self) -> Duration {
        self.started.map_or(Duration::ZERO, |t| t.elapsed())
    }

    fn live(&mut self) -> Option<LiveAudio> {
        let live = LiveAudio::new();
        self.live = Some(live.clone());
        Some(live)
    }
}

impl Drop for PacedCapture {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

/// A capture that "records" a fixed buffer: for tests, and for audio
/// captured by some other part of the app.
///
/// Live, it adds the whole buffer at `start` (16 kHz audio only) and ends
/// at `stop`.
#[derive(Debug, Default)]
pub struct BufferCapture {
    pcm: Pcm,
    started: Option<Instant>,
    live: Option<LiveAudio>,
}

impl BufferCapture {
    /// A capture that yields `pcm` when stopped.
    pub fn new(pcm: Pcm) -> Self {
        BufferCapture {
            pcm,
            started: None,
            live: None,
        }
    }
}

impl AudioCapture for BufferCapture {
    fn start(&mut self) -> Result<(), DictationError> {
        self.started = Some(Instant::now());
        if let Some(live) = &self.live {
            live.push(&to_f32(&self.pcm.samples));
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<Pcm, DictationError> {
        if let Some(live) = &self.live {
            live.end();
        }
        if self.started.take().is_none() {
            return Err(DictationError::NotRecording);
        }
        Ok(std::mem::take(&mut self.pcm))
    }

    fn cancel(&mut self) {
        if let Some(live) = &self.live {
            live.end();
        }
        self.started = None;
        self.pcm = Pcm::default();
    }

    fn elapsed(&self) -> Duration {
        self.started.map_or(Duration::ZERO, |t| t.elapsed())
    }

    fn live(&mut self) -> Option<LiveAudio> {
        if self.pcm.sample_rate != WHISPER_SAMPLE_RATE {
            return None;
        }
        let live = LiveAudio::new();
        self.live = Some(live.clone());
        Some(live)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_is_standard() {
        let pcm = Pcm::new(16_000, vec![0, 1, -1, i16::MAX]);
        let wav = pcm.to_wav();
        assert_eq!(wav.len(), 44 + 8);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes([wav[4], wav[5], wav[6], wav[7]]), 36 + 8);
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(
            u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]),
            16_000
        );
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]), 8);
        assert_eq!(&wav[50..52], &i16::MAX.to_le_bytes());
    }

    #[test]
    fn duration() {
        assert_eq!(
            Pcm::new(16_000, vec![0; 24_000]).duration(),
            Duration::from_millis(1500)
        );
        assert_eq!(Pcm::default().duration(), Duration::ZERO);
    }

    #[test]
    fn buffer_capture_lifecycle() {
        let mut c = BufferCapture::new(Pcm::new(8000, vec![1, 2, 3]));
        assert!(matches!(c.stop(), Err(DictationError::NotRecording)));
        c.start().unwrap();
        assert_eq!(c.stop().unwrap().samples, vec![1, 2, 3]);
        c.start().unwrap();
        c.cancel();
        assert_eq!(c.elapsed(), Duration::ZERO);
    }

    #[test]
    fn writes_wav_files() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.wav");
        Pcm::new(16_000, vec![5; 10]).write_wav(&p).unwrap();
        assert_eq!(std::fs::metadata(&p).unwrap().len(), 64);
    }
}
