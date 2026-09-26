//! Reading the WAV files engines write, and joining them into one.
//!
//! Engines write RIFF/WAVE files with a `fmt ` chunk and a `data` chunk,
//! sometimes with other chunks (`LIST`, `fact`) in between, and sometimes
//! with a `data` size of 0 or `0xFFFFFFFF` when they streamed the file. The
//! reader copes with all of these; the writer keeps the first file's format
//! chunk byte for byte, so any PCM or float format an engine produces passes
//! through unchanged.

use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::ExportError;

/// The format of a WAV file: its `fmt ` chunk and the facts export needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavFormat {
    /// The `fmt ` chunk's body, byte for byte.
    pub fmt_chunk: Vec<u8>,
    /// Format tag (1 = PCM, 3 = IEEE float, 0xFFFE = extensible).
    pub format_tag: u16,
    /// Channels.
    pub channels: u16,
    /// Samples per second.
    pub sample_rate: u32,
    /// Bytes per sample frame (all channels).
    pub block_align: u16,
    /// Bits per sample.
    pub bits_per_sample: u16,
}

impl WavFormat {
    /// 16-bit mono PCM at `sample_rate`.
    pub fn pcm16_mono(sample_rate: u32) -> Self {
        let mut fmt = Vec::with_capacity(16);
        fmt.extend_from_slice(&1u16.to_le_bytes());
        fmt.extend_from_slice(&1u16.to_le_bytes());
        fmt.extend_from_slice(&sample_rate.to_le_bytes());
        fmt.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        fmt.extend_from_slice(&2u16.to_le_bytes());
        fmt.extend_from_slice(&16u16.to_le_bytes());
        WavFormat {
            fmt_chunk: fmt,
            format_tag: 1,
            channels: 1,
            sample_rate,
            block_align: 2,
            bits_per_sample: 16,
        }
    }

    fn parse(body: &[u8]) -> Option<Self> {
        let u16_at = |i: usize| Some(u16::from_le_bytes([*body.get(i)?, *body.get(i + 1)?]));
        let u32_at = |i: usize| {
            Some(u32::from_le_bytes([
                *body.get(i)?,
                *body.get(i + 1)?,
                *body.get(i + 2)?,
                *body.get(i + 3)?,
            ]))
        };
        let f = WavFormat {
            fmt_chunk: body.to_vec(),
            format_tag: u16_at(0)?,
            channels: u16_at(2)?,
            sample_rate: u32_at(4)?,
            block_align: u16_at(12)?,
            bits_per_sample: u16_at(14)?,
        };
        (f.channels > 0 && f.sample_rate > 0 && f.block_align > 0).then_some(f)
    }

    /// The byte value of silence: 0x80 for 8-bit PCM, else 0.
    fn silence_byte(&self) -> u8 {
        if self.format_tag == 1 && self.bits_per_sample == 8 {
            0x80
        } else {
            0
        }
    }

    /// Whole sample frames in `bytes` bytes of audio.
    pub fn frames(&self, bytes: u64) -> u64 {
        bytes / u64::from(self.block_align)
    }

    /// Frames in `ms` milliseconds (rounded down).
    pub fn frames_for_ms(&self, ms: u64) -> u64 {
        ms.saturating_mul(u64::from(self.sample_rate)) / 1000
    }

    /// Milliseconds for `frames` frames, rounded to the nearest.
    pub fn ms(&self, frames: u64) -> u64 {
        let rate = u64::from(self.sample_rate);
        (frames.saturating_mul(1000) + rate / 2) / rate
    }
}

/// A WAV file read into memory: its format and its audio bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavData {
    /// The format.
    pub format: WavFormat,
    /// The audio (the `data` chunk), trimmed to whole frames.
    pub audio: Vec<u8>,
}

impl WavData {
    /// Parses a RIFF/WAVE file.
    pub fn parse(bytes: &[u8], path: &Path) -> Result<Self, ExportError> {
        let bad = |why: &str| ExportError::BadWav {
            path: path.to_owned(),
            reason: why.to_owned(),
        };
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(bad("not a RIFF/WAVE file"));
        }
        let mut format = None;
        let mut i = 12usize;
        while i + 8 <= bytes.len() {
            let id = &bytes[i..i + 4];
            let size = u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]);
            let body_start = i + 8;
            if id == b"data" {
                let format: WavFormat = format.ok_or_else(|| bad("data before fmt"))?;
                let available = bytes.len() - body_start;
                // 0 and 0xFFFFFFFF mean "until the end" in streamed files.
                let len = match size {
                    0 | u32::MAX => available,
                    n => (n as usize).min(available),
                };
                let whole = len - len % usize::from(format.block_align);
                return Ok(WavData {
                    audio: bytes[body_start..body_start + whole].to_vec(),
                    format,
                });
            }
            let end = body_start
                .checked_add(size as usize)
                .filter(|&e| e <= bytes.len())
                .ok_or_else(|| bad("truncated chunk"))?;
            if id == b"fmt " {
                format = Some(
                    WavFormat::parse(&bytes[body_start..end])
                        .ok_or_else(|| bad("unreadable format chunk"))?,
                );
            }
            // Chunks are padded to an even size.
            i = end + (size as usize & 1);
        }
        Err(bad("no data chunk"))
    }

    /// Reads and parses a WAV file.
    pub fn read(path: &Path) -> Result<Self, ExportError> {
        let mut bytes = Vec::new();
        File::open(path)
            .and_then(|mut f| f.read_to_end(&mut bytes))
            .map_err(|e| ExportError::io(path, e))?;
        Self::parse(&bytes, path)
    }

    /// Whole frames of audio.
    pub fn frames(&self) -> u64 {
        self.format.frames(self.audio.len() as u64)
    }
}

/// Writes one WAV file from pieces with the same format, fixing the header
/// sizes when finished.
#[derive(Debug)]
pub struct WavWriter {
    out: BufWriter<File>,
    path: PathBuf,
    format: Option<WavFormat>,
    /// Bytes of audio written so far.
    data_len: u64,
}

/// The largest `data` chunk a RIFF file can describe.
const MAX_DATA: u64 = u32::MAX as u64 - 1024;

impl WavWriter {
    /// Creates (or truncates) `path`. The format comes from the first piece.
    pub fn create(path: &Path) -> Result<Self, ExportError> {
        let file = File::create(path).map_err(|e| ExportError::io(path, e))?;
        Ok(WavWriter {
            out: BufWriter::new(file),
            path: path.to_owned(),
            format: None,
            data_len: 0,
        })
    }

    /// The format, once the first piece fixed it.
    pub fn format(&self) -> Option<&WavFormat> {
        self.format.as_ref()
    }

    /// Frames written so far.
    pub fn frames(&self) -> u64 {
        self.format.as_ref().map_or(0, |f| f.frames(self.data_len))
    }

    fn start(&mut self, format: &WavFormat) -> Result<(), ExportError> {
        let fmt_len = u32::try_from(format.fmt_chunk.len()).unwrap_or(16);
        let mut h = Vec::with_capacity(28 + format.fmt_chunk.len());
        h.extend_from_slice(b"RIFF");
        h.extend_from_slice(&0u32.to_le_bytes()); // patched in finish
        h.extend_from_slice(b"WAVEfmt ");
        h.extend_from_slice(&fmt_len.to_le_bytes());
        h.extend_from_slice(&format.fmt_chunk);
        if fmt_len % 2 == 1 {
            h.push(0);
        }
        h.extend_from_slice(b"data");
        h.extend_from_slice(&0u32.to_le_bytes()); // patched in finish
        self.write(&h)?;
        self.format = Some(format.clone());
        Ok(())
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), ExportError> {
        self.out
            .write_all(bytes)
            .map_err(|e| ExportError::io(&self.path, e))
    }

    /// Appends a piece's audio. Its format must match the first piece's.
    pub fn append(&mut self, piece: &WavData, source: &Path) -> Result<(), ExportError> {
        match &self.format {
            None => self.start(&piece.format)?,
            Some(f) if *f == piece.format => {}
            Some(f) => {
                return Err(ExportError::FormatMismatch {
                    path: source.to_owned(),
                    expected: describe(f),
                    found: describe(&piece.format),
                });
            }
        }
        self.grow(piece.audio.len() as u64)?;
        self.write(&piece.audio)
    }

    /// Appends `ms` milliseconds of silence (nothing before the first
    /// piece, whose format is not known yet).
    pub fn silence(&mut self, ms: u64) -> Result<(), ExportError> {
        let Some(f) = self.format.clone() else {
            return Ok(());
        };
        let bytes = f.frames_for_ms(ms) * u64::from(f.block_align);
        self.grow(bytes)?;
        let chunk = vec![f.silence_byte(); 4096];
        let mut left = bytes;
        while left > 0 {
            let n = left.min(chunk.len() as u64) as usize;
            self.write(&chunk[..n])?;
            left -= n as u64;
        }
        Ok(())
    }

    fn grow(&mut self, bytes: u64) -> Result<(), ExportError> {
        self.data_len += bytes;
        if self.data_len > MAX_DATA {
            return Err(ExportError::TooLong);
        }
        Ok(())
    }

    /// Fixes the header sizes and closes the file. With no pieces at all,
    /// writes an empty 16-bit mono file at 16 kHz.
    pub fn finish(mut self) -> Result<(), ExportError> {
        if self.format.is_none() {
            self.start(&WavFormat::pcm16_mono(16_000))?;
        }
        let fmt_len = self
            .format
            .as_ref()
            .map_or(16, |f| f.fmt_chunk.len() as u64);
        let fmt_padded = fmt_len + (fmt_len & 1);
        let data_len = self.data_len;
        if data_len & 1 == 1 {
            self.write(&[0])?;
        }
        let riff_len = 4 + (8 + fmt_padded) + 8 + data_len + (data_len & 1);
        let path = self.path.clone();
        let io = |e| ExportError::io(&path, e);
        let mut file = self.out.into_inner().map_err(|e| io(e.into_error()))?;
        file.seek(SeekFrom::Start(4)).map_err(io)?;
        file.write_all(&(riff_len as u32).to_le_bytes())
            .map_err(io)?;
        file.seek(SeekFrom::Start(12 + 8 + fmt_padded + 4))
            .map_err(io)?;
        file.write_all(&(data_len as u32).to_le_bytes())
            .map_err(io)?;
        file.flush().map_err(io)
    }
}

fn describe(f: &WavFormat) -> String {
    let kind = match f.format_tag {
        1 => "PCM",
        3 => "float",
        0xFFFE => "extensible",
        _ => "encoded",
    };
    let ch = if f.channels == 1 {
        "mono".to_owned()
    } else {
        format!("{} channels", f.channels)
    };
    format!("{} Hz {}-bit {kind} {ch}", f.sample_rate, f.bits_per_sample)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(rate: u32, samples: &[i16]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + samples.len() as u32 * 2).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&WavFormat::pcm16_mono(rate).fmt_chunk);
        v.extend_from_slice(b"data");
        v.extend_from_slice(&(samples.len() as u32 * 2).to_le_bytes());
        for s in samples {
            v.extend_from_slice(&s.to_le_bytes());
        }
        v
    }

    #[test]
    fn parses_plain_and_odd_files() {
        let p = Path::new("x.wav");
        let d = WavData::parse(&wav(8000, &[1, 2, 3]), p).unwrap();
        assert_eq!(d.format.sample_rate, 8000);
        assert_eq!(d.frames(), 3);
        // A LIST chunk before the data, and a streamed (0xFFFFFFFF) size.
        let mut v = wav(8000, &[5, 6]);
        let data_at = v.len() - 4 - 8;
        let mut list = b"LIST".to_vec();
        list.extend_from_slice(&3u32.to_le_bytes());
        list.extend_from_slice(b"abc\0");
        v.splice(data_at..data_at, list);
        let n = v.len();
        v[n - 8..n - 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let d = WavData::parse(&v, p).unwrap();
        assert_eq!(d.audio, [5, 0, 6, 0]);
        assert!(WavData::parse(b"RIFF\0\0\0\0WAVE", p).is_err());
        assert!(WavData::parse(b"hello", p).is_err());
    }

    #[test]
    fn writer_joins_pieces_and_fixes_sizes() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out.wav");
        let a = WavData::parse(&wav(1000, &[1, 2]), Path::new("a")).unwrap();
        let mut w = WavWriter::create(&out).unwrap();
        w.silence(5).unwrap(); // ignored: no format yet
        w.append(&a, Path::new("a")).unwrap();
        w.silence(3).unwrap(); // 3 frames at 1 kHz
        w.append(&a, Path::new("a")).unwrap();
        assert_eq!(w.frames(), 7);
        w.finish().unwrap();
        let back = WavData::read(&out).unwrap();
        assert_eq!(back.audio, [1, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 0]);
        let bytes = std::fs::read(&out).unwrap();
        let riff = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        assert_eq!(riff as usize, bytes.len() - 8);
        let other = WavData::parse(&wav(2000, &[1]), Path::new("b")).unwrap();
        let mut w = WavWriter::create(&out).unwrap();
        w.append(&a, Path::new("a")).unwrap();
        let err = w.append(&other, Path::new("b")).unwrap_err();
        assert!(err.to_string().contains("2000 Hz"), "{err}");
    }

    #[test]
    fn ms_rounds_to_nearest() {
        let f = WavFormat::pcm16_mono(22050);
        assert_eq!(f.ms(22050), 1000);
        assert_eq!(f.ms(11), 0);
        assert_eq!(f.ms(12), 1);
        assert_eq!(f.frames_for_ms(1000), 22050);
    }
}
