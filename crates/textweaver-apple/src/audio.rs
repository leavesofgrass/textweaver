//! PCM helpers: WAV writing and AIFF reading.
//!
//! `NSSpeechSynthesizer` writes AIFF when speaking to a file; textweaver
//! exports WAV, so [`read_aiff`] decodes the AIFF and [`write_wav`] writes
//! 16-bit PCM WAV. `AVSpeechSynthesizer` hands over PCM buffers, which go
//! straight to [`write_wav`]. Pure Rust, tested on every platform.

use std::io::Write;
use std::path::Path;

/// Mono or multichannel PCM audio as `f32` samples in `-1.0..=1.0`,
/// interleaved.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pcm {
    /// Samples per second.
    pub sample_rate: u32,
    /// Channels (interleaved in `samples`).
    pub channels: u16,
    /// Interleaved samples.
    pub samples: Vec<f32>,
}

impl Pcm {
    /// Frames (samples per channel).
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }

    /// Duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            return 0;
        }
        (self.frames() as u64).saturating_mul(1000) / u64::from(self.sample_rate)
    }
}

/// Audio file failures.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// The file is not AIFF, or uses an encoding this reader does not
    /// support.
    #[error("unsupported audio file: {0}")]
    Unsupported(String),
    /// Reading or writing failed.
    #[error("audio i/o: {0}")]
    Io(#[from] std::io::Error),
}

/// Converts a sample to 16-bit PCM, clamping.
pub fn to_i16(sample: f32) -> i16 {
    let s = (sample.clamp(-1.0, 1.0) * 32767.0).round();
    // The clamp keeps `s` inside i16 range.
    s as i16
}

/// Encodes `pcm` as a 16-bit PCM WAV file.
pub fn wav_bytes(pcm: &Pcm) -> Vec<u8> {
    let channels = pcm.channels.max(1);
    let data_len = u32::try_from(pcm.samples.len() * 2).unwrap_or(u32::MAX - 36);
    let block_align = channels * 2;
    let byte_rate = pcm.sample_rate * u32::from(block_align);
    let mut out = Vec::with_capacity(44 + pcm.samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&pcm.sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in &pcm.samples {
        out.extend_from_slice(&to_i16(*s).to_le_bytes());
    }
    out
}

/// Writes `pcm` to `path` as a 16-bit PCM WAV file.
pub fn write_wav(path: &Path, pcm: &Pcm) -> Result<(), AudioError> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(&wav_bytes(pcm))?;
    f.flush()?;
    Ok(())
}

/// Decodes an 80-bit IEEE 754 extended float (AIFF's sample rate).
fn extended_to_f64(b: &[u8; 10]) -> f64 {
    let sign = if b[0] & 0x80 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from(u16::from_be_bytes([b[0] & 0x7F, b[1]]));
    let mut mantissa = 0u64;
    for &byte in &b[2..10] {
        mantissa = (mantissa << 8) | u64::from(byte);
    }
    if exponent == 0 && mantissa == 0 {
        return 0.0;
    }
    sign * (mantissa as f64) * 2f64.powi(exponent - 16383 - 63)
}

fn be_u16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn be_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Decodes AIFF or AIFF-C (uncompressed big-endian `NONE`/`twos`,
/// little-endian `sowt`, or 32-bit float `fl32`) with 8, 16, 24, or 32-bit
/// integer samples.
pub fn read_aiff(bytes: &[u8]) -> Result<Pcm, AudioError> {
    let bad = |m: &str| AudioError::Unsupported(m.to_string());
    if bytes.len() < 12 || &bytes[0..4] != b"FORM" {
        return Err(bad("not an IFF file"));
    }
    let aifc = match &bytes[8..12] {
        b"AIFF" => false,
        b"AIFC" => true,
        _ => return Err(bad("not AIFF")),
    };
    let mut pos = 12;
    let mut channels = 0u16;
    let mut bits = 0u16;
    let mut rate = 0f64;
    let mut compression = *b"NONE";
    let mut sound: Option<&[u8]> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let len = be_u32(bytes, pos + 4).ok_or_else(|| bad("truncated chunk"))? as usize;
        let body_start = pos + 8;
        let body_end = body_start.saturating_add(len).min(bytes.len());
        let body = &bytes[body_start..body_end];
        match id {
            b"COMM" => {
                channels = be_u16(body, 0).ok_or_else(|| bad("short COMM"))?;
                bits = be_u16(body, 6).ok_or_else(|| bad("short COMM"))?;
                let ext: [u8; 10] = body
                    .get(8..18)
                    .and_then(|s| s.try_into().ok())
                    .ok_or_else(|| bad("short COMM"))?;
                rate = extended_to_f64(&ext);
                if aifc && let Some(c) = body.get(18..22) {
                    compression.copy_from_slice(c);
                }
            }
            b"SSND" => {
                let offset = be_u32(body, 0).ok_or_else(|| bad("short SSND"))? as usize;
                sound = body.get(8 + offset..);
            }
            _ => {}
        }
        // Chunks are padded to even lengths.
        pos = body_start.saturating_add(len).saturating_add(len & 1);
    }
    let data = sound.ok_or_else(|| bad("no sound data"))?;
    if channels == 0 || rate <= 0.0 {
        return Err(bad("missing format"));
    }
    let samples: Vec<f32> = match (&compression, bits) {
        (b"fl32" | b"FL32", _) => data
            .chunks_exact(4)
            .map(|c| f32::from_be_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
        (b"NONE" | b"twos", 8) => data.iter().map(|&b| f32::from(b as i8) / 128.0).collect(),
        (b"NONE" | b"twos", 16) => data
            .chunks_exact(2)
            .map(|c| f32::from(i16::from_be_bytes([c[0], c[1]])) / 32768.0)
            .collect(),
        (b"sowt", 16) => data
            .chunks_exact(2)
            .map(|c| f32::from(i16::from_le_bytes([c[0], c[1]])) / 32768.0)
            .collect(),
        (b"NONE" | b"twos", 24) => data
            .chunks_exact(3)
            .map(|c| (i32::from_be_bytes([c[0], c[1], c[2], 0]) >> 8) as f32 / 8_388_608.0)
            .collect(),
        (b"NONE" | b"twos", 32) => data
            .chunks_exact(4)
            .map(|c| i32::from_be_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0)
            .collect(),
        (c, b) => {
            return Err(AudioError::Unsupported(format!(
                "compression {} with {b} bits",
                String::from_utf8_lossy(c)
            )));
        }
    };
    Ok(Pcm {
        sample_rate: rate.round() as u32,
        channels,
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encodes `rate` as an 80-bit extended float.
    fn extended(rate: u32) -> [u8; 10] {
        let mut out = [0u8; 10];
        let shift = rate.leading_zeros();
        let exponent = 16383 + 31 - shift as u16;
        out[0..2].copy_from_slice(&exponent.to_be_bytes());
        let mantissa = (u64::from(rate)) << (32 + shift);
        out[2..10].copy_from_slice(&mantissa.to_be_bytes());
        out
    }

    fn aiff(samples: &[i16], rate: u32, aifc: Option<&[u8; 4]>) -> Vec<u8> {
        let mut comm = Vec::new();
        comm.extend_from_slice(&1u16.to_be_bytes());
        comm.extend_from_slice(&(samples.len() as u32).to_be_bytes());
        comm.extend_from_slice(&16u16.to_be_bytes());
        comm.extend_from_slice(&extended(rate));
        if let Some(c) = aifc {
            comm.extend_from_slice(c);
            comm.extend_from_slice(&[0, 0]); // empty pstring, padded
        }
        let mut ssnd = vec![0u8; 8];
        for s in samples {
            if aifc == Some(b"sowt") {
                ssnd.extend_from_slice(&s.to_le_bytes());
            } else {
                ssnd.extend_from_slice(&s.to_be_bytes());
            }
        }
        let mut body = Vec::new();
        body.extend_from_slice(if aifc.is_some() { b"AIFC" } else { b"AIFF" });
        if aifc.is_some() {
            body.extend_from_slice(b"FVER");
            body.extend_from_slice(&4u32.to_be_bytes());
            body.extend_from_slice(&0xA280_5140u32.to_be_bytes());
        }
        body.extend_from_slice(b"COMM");
        body.extend_from_slice(&(comm.len() as u32).to_be_bytes());
        body.extend_from_slice(&comm);
        body.extend_from_slice(b"SSND");
        body.extend_from_slice(&(ssnd.len() as u32).to_be_bytes());
        body.extend_from_slice(&ssnd);
        let mut out = b"FORM".to_vec();
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn extended_floats_decode_common_rates() {
        for rate in [8000u32, 11025, 16000, 22050, 44100, 48000] {
            assert_eq!(extended_to_f64(&extended(rate)), f64::from(rate));
        }
        // 22050 Hz as written by Apple's speech synthesizer.
        let apple = [0x40, 0x0D, 0xAC, 0x44, 0, 0, 0, 0, 0, 0];
        assert_eq!(extended_to_f64(&apple), 22050.0);
    }

    #[test]
    fn aiff_and_aifc_decode() {
        let samples = [0i16, 16384, -16384, 32767, -32768];
        for kind in [None, Some(b"NONE"), Some(b"sowt")] {
            let pcm = read_aiff(&aiff(&samples, 22050, kind)).expect("decodes");
            assert_eq!(pcm.sample_rate, 22050);
            assert_eq!(pcm.channels, 1);
            assert_eq!(pcm.samples.len(), 5);
            assert_eq!(pcm.samples[1], 0.5);
            assert_eq!(pcm.samples[2], -0.5);
            assert_eq!(pcm.samples[4], -1.0);
        }
    }

    #[test]
    fn aiff_rejects_other_files() {
        assert!(read_aiff(b"RIFF\0\0\0\0WAVE").is_err());
        assert!(read_aiff(b"").is_err());
        let mut bad = aiff(&[1, 2], 16000, Some(b"ima4"));
        assert!(matches!(read_aiff(&bad), Err(AudioError::Unsupported(_))));
        bad.truncate(20);
        assert!(read_aiff(&bad).is_err());
    }

    #[test]
    fn wav_header_and_samples() {
        let pcm = Pcm {
            sample_rate: 16000,
            channels: 1,
            samples: vec![0.0, 0.5, -1.0, 2.0],
        };
        let b = wav_bytes(&pcm);
        assert_eq!(&b[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(b[4..8].try_into().unwrap()), 36 + 8);
        assert_eq!(&b[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(b[24..28].try_into().unwrap()), 16000);
        assert_eq!(u32::from_le_bytes(b[28..32].try_into().unwrap()), 32000);
        assert_eq!(&b[36..40], b"data");
        assert_eq!(u32::from_le_bytes(b[40..44].try_into().unwrap()), 8);
        let s: Vec<i16> = b[44..]
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        assert_eq!(s, [0, 16384, -32767, 32767]);
        assert_eq!(pcm.duration_ms(), 0);
        assert_eq!(pcm.frames(), 4);
    }

    #[test]
    fn wav_writes_to_a_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a.wav");
        let pcm = Pcm {
            sample_rate: 8000,
            channels: 1,
            samples: vec![0.25; 8000],
        };
        write_wav(&path, &pcm).expect("writes");
        let len = std::fs::metadata(&path).expect("exists").len();
        assert_eq!(len, 44 + 16000);
        assert_eq!(pcm.duration_ms(), 1000);
    }
}
