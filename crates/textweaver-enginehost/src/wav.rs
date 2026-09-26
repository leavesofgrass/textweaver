//! Minimal WAV (RIFF, 16-bit PCM, mono) writing for `synthesize_to_file`,
//! and the volume gain applied to synthesized audio.

use std::io::{self, Write};
use std::path::Path;

/// Encodes `samples` as a 16-bit mono PCM WAV file image. Audio longer than
/// a WAV can describe (about 2 GiB) is cut to fit.
pub fn encode(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let max = (u32::MAX as usize - 36) / 2;
    let samples = &samples[..samples.len().min(max)];
    let data_len = u32::try_from(samples.len() * 2).unwrap_or(u32::MAX - 36);
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&sample_rate.saturating_mul(2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Writes `samples` as a 16-bit mono WAV at `sample_rate` Hz to `w`.
pub fn write_wav(w: &mut impl Write, sample_rate: u32, samples: &[i16]) -> io::Result<()> {
    if samples.len() > (u32::MAX as usize - 36) / 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "audio too long for WAV",
        ));
    }
    w.write_all(&encode(samples, sample_rate))?;
    w.flush()
}

/// Writes `samples` to a new file at `path` as a WAV.
pub fn write(path: &Path, samples: &[i16], sample_rate: u32) -> io::Result<()> {
    let mut f = io::BufWriter::new(std::fs::File::create(path)?);
    write_wav(&mut f, sample_rate, samples)
}

/// Parses a WAV written by [`encode`]: `(sample_rate, samples)`.
pub fn read_wav(bytes: &[u8]) -> Option<(u32, Vec<i16>)> {
    if bytes.len() < 44 || &bytes[..4] != b"RIFF" || &bytes[8..16] != b"WAVEfmt " {
        return None;
    }
    let rate = u32::from_le_bytes(bytes[24..28].try_into().ok()?);
    let len = u32::from_le_bytes(bytes[40..44].try_into().ok()?) as usize;
    let data = bytes.get(44..44 + len)?;
    Some((
        rate,
        data.chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect(),
    ))
}

/// Scales samples by a linear `gain` (volume), rounding and clamping to
/// the 16-bit range. A gain of 1 returns the samples unchanged.
pub fn apply_gain(samples: Vec<i16>, gain: f32) -> Vec<i16> {
    if (gain - 1.0).abs() < f32::EPSILON {
        return samples;
    }
    samples
        .iter()
        .map(|&s| (f32::from(s) * gain).round().clamp(-32768.0, 32767.0) as i16)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let mut out = Vec::new();
        write_wav(&mut out, 11025, &[0, 1, -1, 300]).unwrap();
        assert_eq!(out.len(), 44 + 8);
        assert_eq!(read_wav(&out), Some((11025, vec![0, 1, -1, 300])));
    }

    #[test]
    fn header_describes_the_data() {
        let w = encode(&[1, -2, 3], 22050);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(w[4..8].try_into().unwrap()), 36 + 6);
        assert_eq!(&w[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), 22050);
        assert_eq!(u32::from_le_bytes(w[28..32].try_into().unwrap()), 44100);
        assert_eq!(&w[36..40], b"data");
        assert_eq!(u32::from_le_bytes(w[40..44].try_into().unwrap()), 6);
        assert_eq!(w.len(), 44 + 6);
        assert_eq!(i16::from_le_bytes([w[46], w[47]]), -2);
    }

    #[test]
    fn files_are_written_whole() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.wav");
        write(&path, &[5, -5], 8000).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(read_wav(&bytes), Some((8000, vec![5, -5])));
        assert_eq!(read_wav(b"RIFF"), None);
    }

    #[test]
    fn gain_rounds_and_clamps() {
        assert_eq!(apply_gain(vec![100, -100], 1.0), [100, -100]);
        assert_eq!(apply_gain(vec![3, -3, 16384], 0.5), [2, -2, 8192]);
        assert_eq!(apply_gain(vec![30000, -30000], 2.0), [32767, -32768]);
    }
}
