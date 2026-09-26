//! Audio to dictate from: recorded samples and the capture trait a
//! microphone implements.
//!
//! Microphone capture itself needs an audio input library (`cpal`), which
//! is not in the workspace yet (ADR-0013); the app plugs a microphone in
//! through [`AudioCapture`] when it is. [`BufferCapture`] serves tests and
//! audio recorded elsewhere.

use std::io::Write;
use std::path::Path;
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
/// between `start` and `stop`, with no fixed length (Star's
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
}

/// A capture that "records" a fixed buffer: for tests, and for audio
/// captured by some other part of the app.
#[derive(Debug, Default)]
pub struct BufferCapture {
    pcm: Pcm,
    started: Option<Instant>,
}

impl BufferCapture {
    /// A capture that yields `pcm` when stopped.
    pub fn new(pcm: Pcm) -> Self {
        BufferCapture { pcm, started: None }
    }
}

impl AudioCapture for BufferCapture {
    fn start(&mut self) -> Result<(), DictationError> {
        self.started = Some(Instant::now());
        Ok(())
    }

    fn stop(&mut self) -> Result<Pcm, DictationError> {
        if self.started.take().is_none() {
            return Err(DictationError::NotRecording);
        }
        Ok(std::mem::take(&mut self.pcm))
    }

    fn cancel(&mut self) {
        self.started = None;
        self.pcm = Pcm::default();
    }

    fn elapsed(&self) -> Duration {
        self.started.map_or(Duration::ZERO, |t| t.elapsed())
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
