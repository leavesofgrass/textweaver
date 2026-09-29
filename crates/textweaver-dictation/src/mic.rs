//! Microphone capture with rodio's `recording` feature (feature `mic`).
//!
//! The input stream lives on its own thread (a `cpal` stream is not
//! `Send` on every platform), which drains rodio's small ring buffer
//! continuously and mixes the channels down to mono. When recording
//! stops, the audio is resampled to Whisper's 16 kHz with rubato. Live
//! ([`AudioCapture::live`]), the thread also resamples as it records and
//! hands the 16 kHz audio on in blocks of about a tenth of a second.
//!
//! Tests never open a microphone; [`MicCapture`] is exercised by hand
//! (`tw dictate` without `--file`, and the listening checklist in
//! `docs/releasing.md`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rodio::Source;
use rodio::microphone::MicrophoneBuilder;

use crate::DictationError;
use crate::audio::{StreamResampler, resample};
use crate::capture::{AudioCapture, LiveAudio, Pcm, WHISPER_SAMPLE_RATE};

/// Recorded audio shared with the capture thread.
#[derive(Default)]
struct Shared {
    samples: Vec<f32>,
    rate: u32,
    error: Option<String>,
}

/// The default microphone.
#[derive(Default)]
pub struct MicCapture {
    stop: Arc<AtomicBool>,
    shared: Arc<Mutex<Shared>>,
    thread: Option<JoinHandle<()>>,
    started: Option<Instant>,
    live: Option<LiveAudio>,
}

impl std::fmt::Debug for MicCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MicCapture")
            .field("recording", &self.thread.is_some())
            .finish_non_exhaustive()
    }
}

impl MicCapture {
    /// A capture from the system's default input device.
    pub fn new() -> Self {
        MicCapture::default()
    }

    fn finish(&mut self) -> Result<(Vec<f32>, u32), DictationError> {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        self.started = None;
        let mut s = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(e) = s.error.take() {
            return Err(DictationError::Capture(e));
        }
        Ok((std::mem::take(&mut s.samples), s.rate))
    }
}

impl AudioCapture for MicCapture {
    fn start(&mut self) -> Result<(), DictationError> {
        if self.thread.is_some() {
            return Err(DictationError::Busy);
        }
        self.stop = Arc::new(AtomicBool::new(false));
        self.shared = Arc::new(Mutex::new(Shared::default()));
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let stop = Arc::clone(&self.stop);
        let shared = Arc::clone(&self.shared);
        let live = self.live.clone();
        let thread = std::thread::Builder::new()
            .name("textweaver-microphone".into())
            .spawn(move || {
                record(&stop, &shared, live.as_ref(), &ready_tx);
                if let Some(live) = &live {
                    live.end();
                }
            })
            .map_err(|e| DictationError::Capture(e.to_string()))?;
        match ready_rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(())) => {
                self.thread = Some(thread);
                self.started = Some(Instant::now());
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(DictationError::Capture(e))
            }
            Err(_) => {
                self.stop.store(true, Ordering::SeqCst);
                Err(DictationError::Capture(
                    "The microphone did not open within 10 seconds.".into(),
                ))
            }
        }
    }

    fn stop(&mut self) -> Result<Pcm, DictationError> {
        if self.thread.is_none() {
            return Err(DictationError::NotRecording);
        }
        let (samples, rate) = self.finish()?;
        let samples = if rate == 0 {
            Vec::new()
        } else {
            resample(&samples, rate, WHISPER_SAMPLE_RATE).map_err(DictationError::Capture)?
        };
        let pcm = samples
            .iter()
            // In range after the clamp.
            .map(|&s| (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16)
            .collect();
        Ok(Pcm::new(WHISPER_SAMPLE_RATE, pcm))
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

impl Drop for MicCapture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The capture thread: opens the default input, then collects mono
/// samples until `stop`, and hands them on live at 16 kHz when asked.
fn record(
    stop: &AtomicBool,
    shared: &Mutex<Shared>,
    live: Option<&LiveAudio>,
    ready: &std::sync::mpsc::Sender<Result<(), String>>,
) {
    let opened = MicrophoneBuilder::new()
        .default_device()
        .and_then(|b| b.default_config())
        .map_err(|e| format!("No microphone was found: {e}"))
        .and_then(|b| {
            b.open_stream()
                .map_err(|e| format!("The microphone could not be opened: {e}"))
        });
    let mut mic = match opened {
        Ok(m) => m,
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };
    let channels = usize::from(mic.channels().get());
    let rate = mic.sample_rate().get();
    shared.lock().unwrap_or_else(|e| e.into_inner()).rate = rate;
    let mut live = match live.map(|l| StreamResampler::new(rate).map(|r| (l, r))) {
        None => None,
        Some(Ok(l)) => Some(l),
        Some(Err(e)) => {
            let _ = ready.send(Err(e));
            return;
        }
    };
    let _ = ready.send(Ok(()));
    // Hands a block on live, resampled; a failure ends the live audio,
    // and the recording still reaches `stop`.
    let mut hand_on = |block: &[f32]| {
        if let Some((l, r)) = &mut live {
            match r.push(block) {
                Ok(out) => l.push(&out),
                Err(e) => {
                    log::warn!("dictation: live resampling failed: {e}");
                    l.end();
                }
            }
        }
    };
    // About a tenth of a second at 44.1 or 48 kHz, so live dictation
    // hears the audio soon after it is spoken.
    let block_len = (rate as usize / 10).clamp(256, 4096);
    let mut frame = Vec::with_capacity(channels);
    let mut block = Vec::with_capacity(block_len);
    while !stop.load(Ordering::SeqCst) {
        // `next` waits for the device; it ends only on a stream error.
        let Some(sample) = mic.next() else {
            shared.lock().unwrap_or_else(|e| e.into_inner()).error =
                Some("The microphone stopped working.".into());
            break;
        };
        frame.push(sample);
        if frame.len() == channels {
            block.push(frame.iter().sum::<f32>() / channels as f32);
            frame.clear();
        }
        if block.len() >= block_len {
            hand_on(&block);
            shared
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .samples
                .append(&mut block);
        }
    }
    hand_on(&block);
    shared
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .samples
        .append(&mut block);
    if let Some((l, r)) = &mut live {
        match r.finish() {
            Ok(out) => l.push(&out),
            Err(e) => log::warn!("dictation: live resampling failed: {e}"),
        }
    }
}
