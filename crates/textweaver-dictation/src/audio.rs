//! Audio for in-process Whisper: reading WAV files and resampling to
//! Whisper's 16 kHz with rubato (features `rten` and `mic`).

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use crate::capture::WHISPER_SAMPLE_RATE;

/// Mono audio as floats in `-1.0..=1.0`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MonoAudio {
    /// Samples per second.
    pub sample_rate: u32,
    /// The samples.
    pub samples: Vec<f32>,
}

/// Reads a WAV file's audio, mixed down to mono: 8, 16, 24, or 32-bit
/// integer PCM, or 32-bit float, including `WAVE_FORMAT_EXTENSIBLE`
/// headers. Other formats (compressed audio) are refused with a sentence
/// that says so.
pub fn read_wav(bytes: &[u8]) -> Result<MonoAudio, String> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("This is not a WAV file. Convert it to WAV first.".into());
    }
    let mut pos = 12;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let len = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        let body_start = pos + 8;
        let body_end = body_start.saturating_add(len).min(bytes.len());
        let body = &bytes[body_start..body_end];
        match id {
            b"fmt " if body.len() >= 16 => {
                let mut tag = u16::from_le_bytes([body[0], body[1]]);
                let channels = u16::from_le_bytes([body[2], body[3]]);
                let rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
                let bits = u16::from_le_bytes([body[14], body[15]]);
                // WAVE_FORMAT_EXTENSIBLE: the real tag starts the sub-format GUID.
                if tag == 0xFFFE && body.len() >= 26 {
                    tag = u16::from_le_bytes([body[24], body[25]]);
                }
                fmt = Some((tag, channels, rate, bits));
            }
            b"data" => data = Some(body),
            _ => {}
        }
        // Chunks are padded to an even length.
        pos = body_start.saturating_add(len).saturating_add(len & 1);
    }
    let (tag, channels, rate, bits) = fmt.ok_or("The WAV file has no format chunk.")?;
    let data = data.ok_or("The WAV file has no audio data.")?;
    if channels == 0 || rate == 0 {
        return Err("The WAV file's format is not valid.".into());
    }
    let width = usize::from(bits / 8);
    let sample = |b: &[u8]| -> Option<f32> {
        Some(match (tag, bits) {
            (1, 8) => (f32::from(b[0]) - 128.0) / 128.0,
            (1, 16) => f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0,
            (1, 24) => (i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8) as f32 / 8_388_608.0,
            (1, 32) => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32 / 2_147_483_648.0,
            (3, 32) => f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            _ => return None,
        })
    };
    if width == 0 || sample(&[0, 0, 0, 0]).is_none() {
        return Err(format!(
            "WAV audio in format {tag} with {bits}-bit samples is not supported. Convert it to 16-bit PCM."
        ));
    }
    let frame = width * usize::from(channels);
    let samples = data
        .chunks_exact(frame)
        .map(|f| {
            let sum: f32 = f.chunks_exact(width).filter_map(sample).sum();
            sum / f32::from(channels)
        })
        .collect();
    Ok(MonoAudio {
        sample_rate: rate,
        samples,
    })
}

/// Resamples `audio` to 16 kHz (unchanged when it already is).
pub fn to_whisper_rate(audio: &MonoAudio) -> Result<Vec<f32>, String> {
    resample(&audio.samples, audio.sample_rate, WHISPER_SAMPLE_RATE)
}

/// Resamples mono `samples` from `from` Hz to `to` Hz with rubato's FFT
/// resampler.
pub fn resample(samples: &[f32], from: u32, to: u32) -> Result<Vec<f32>, String> {
    if from == to || samples.is_empty() {
        return Ok(samples.to_vec());
    }
    let mut r = Fft::<f32>::new(from as usize, to as usize, 1024, 1, FixedSync::Input)
        .map_err(|e| format!("cannot resample {from} Hz audio: {e}"))?;
    let input = InterleavedSlice::new(samples, 1, samples.len())
        .map_err(|e| format!("cannot resample: {e}"))?;
    let out = r
        .process_all(&input, samples.len(), None)
        .map_err(|e| format!("cannot resample: {e}"))?;
    Ok(out.take_data())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(tag: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * u32::from(channels * bits / 8)).to_le_bytes());
        out.extend_from_slice(&(channels * bits / 8).to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        // An unknown chunk before the data, with odd length and padding.
        out.extend_from_slice(b"LIST");
        out.extend_from_slice(&3u32.to_le_bytes());
        out.extend_from_slice(&[1, 2, 3, 0]);
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn reads_16_bit_stereo_as_mono() {
        let mut data = Vec::new();
        for (l, r) in [(16384i16, 0i16), (-32768, -32768)] {
            data.extend_from_slice(&l.to_le_bytes());
            data.extend_from_slice(&r.to_le_bytes());
        }
        let a = read_wav(&wav(1, 2, 44_100, 16, &data)).unwrap();
        assert_eq!(a.sample_rate, 44_100);
        assert_eq!(a.samples, vec![0.25, -1.0]);
    }

    #[test]
    fn reads_float_and_24_bit() {
        let a = read_wav(&wav(3, 1, 16_000, 32, &0.5f32.to_le_bytes())).unwrap();
        assert_eq!(a.samples, vec![0.5]);
        let a = read_wav(&wav(1, 1, 16_000, 24, &[0, 0, 0x40])).unwrap();
        assert_eq!(a.samples, vec![0.5]);
        let a = read_wav(&wav(1, 1, 8_000, 8, &[128, 0])).unwrap();
        assert_eq!(a.samples, vec![0.0, -1.0]);
    }

    #[test]
    fn refuses_what_it_cannot_read() {
        assert!(read_wav(b"ID3\x04 mp3").unwrap_err().contains("not a WAV"));
        let e = read_wav(&wav(0x55, 1, 16_000, 16, &[0, 0])).unwrap_err();
        assert!(e.contains("not supported"), "{e}");
    }

    #[test]
    fn resamples_to_16_khz() {
        let tone: Vec<f32> = (0..48_000)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin())
            .collect();
        let out = resample(&tone, 48_000, 16_000).unwrap();
        assert!((out.len() as i64 - 16_000).abs() <= 2, "{}", out.len());
        // Still a 440 Hz tone: count zero crossings over the middle second.
        let crossings = out[1000..15_000]
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        assert!((380..=390).contains(&crossings), "{crossings}");
        assert_eq!(resample(&[1.0], 16_000, 16_000).unwrap(), vec![1.0]);
    }
}
