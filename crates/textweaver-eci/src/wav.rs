//! Minimal WAV (RIFF, 16-bit PCM, mono) writing.

use std::io::{self, Write};

/// Writes `samples` as a 16-bit mono WAV at `sample_rate` Hz.
pub fn write_wav(w: &mut impl Write, sample_rate: u32, samples: &[i16]) -> io::Result<()> {
    let data_len = u32::try_from(samples.len() * 2)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "audio too long for WAV"))?;
    let mut h = Vec::with_capacity(44);
    h.extend_from_slice(b"RIFF");
    h.extend_from_slice(&(36 + data_len).to_le_bytes());
    h.extend_from_slice(b"WAVEfmt ");
    h.extend_from_slice(&16u32.to_le_bytes());
    h.extend_from_slice(&1u16.to_le_bytes()); // PCM
    h.extend_from_slice(&1u16.to_le_bytes()); // mono
    h.extend_from_slice(&sample_rate.to_le_bytes());
    h.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    h.extend_from_slice(&2u16.to_le_bytes()); // block align
    h.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    h.extend_from_slice(b"data");
    h.extend_from_slice(&data_len.to_le_bytes());
    w.write_all(&h)?;
    let mut body = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        body.extend_from_slice(&s.to_le_bytes());
    }
    w.write_all(&body)?;
    w.flush()
}

/// Parses a WAV written by [`write_wav`]: `(sample_rate, samples)`.
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
}
