//! Minimal WAV writing shared by the backends that write files.

/// A 16-bit mono PCM WAV file holding `samples`.
pub fn wav_bytes(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let mut out = header(samples.len(), sample_rate);
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// A 16-bit mono PCM WAV file of `samples` samples of silence.
pub fn silent_wav(samples: usize, sample_rate: u32) -> Vec<u8> {
    let mut out = header(samples, sample_rate);
    out.resize(out.len() + samples * 2, 0);
    out
}

/// The 44-byte header of a 16-bit mono PCM WAV file with `samples` samples.
fn header(samples: usize, sample_rate: u32) -> Vec<u8> {
    let data_len = u32::try_from(samples * 2).unwrap_or(u32::MAX - 36);
    let mut out = Vec::with_capacity(44 + samples * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36u32.saturating_add(data_len)).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header() {
        let w = wav_bytes(&[0, 1, -1], 22050);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[8..16], b"WAVEfmt ");
        assert_eq!(w.len(), 44 + 6);
        assert_eq!(u32::from_le_bytes([w[24], w[25], w[26], w[27]]), 22050);
        assert_eq!(u32::from_le_bytes([w[40], w[41], w[42], w[43]]), 6);
    }

    #[test]
    fn silence_is_zeros() {
        let w = silent_wav(4, 16_000);
        assert_eq!(w.len(), 44 + 8);
        assert!(w[44..].iter().all(|&b| b == 0));
        assert_eq!(w[..44], wav_bytes(&[0; 4], 16_000)[..44]);
    }
}
