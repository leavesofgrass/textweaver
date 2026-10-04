//! Reading the joined WAV back in blocks, for the in-process encoders
//! (FLAC, MP3, Opus, Ogg Vorbis).
//!
//! Engines write 8, 16, 24 and 32-bit PCM and 32 and 64-bit float; each
//! sample is read as an integer of at most 24 bits
//! ([`PcmReader::bits`]): 32-bit PCM keeps its top 24 bits and float
//! becomes 16-bit PCM, since FLAC stores integers of at most 24 bits and
//! LAME takes 16-bit samples.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::ExportError;

/// Bytes read at once, at most.
const READ_LIMIT: usize = 1 << 20;

/// How samples are stored in the WAV being read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SampleKind {
    /// Unsigned 8-bit PCM.
    U8,
    /// Signed little-endian PCM of 2, 3 or 4 bytes.
    Int(usize),
    /// IEEE float of 4 or 8 bytes.
    Float(usize),
}

impl SampleKind {
    pub(crate) fn bytes(self) -> usize {
        match self {
            SampleKind::U8 => 1,
            SampleKind::Int(n) | SampleKind::Float(n) => n,
        }
    }

    /// Bits per sample as read.
    pub(crate) fn bits(self) -> usize {
        match self {
            SampleKind::U8 => 8,
            SampleKind::Int(2) | SampleKind::Float(_) => 16,
            SampleKind::Int(_) => 24,
        }
    }

    pub(crate) fn convert(self, b: &[u8]) -> i32 {
        match self {
            SampleKind::U8 => i32::from(b[0]) - 128,
            SampleKind::Int(2) => i32::from(i16::from_le_bytes([b[0], b[1]])),
            SampleKind::Int(3) => i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8,
            SampleKind::Int(_) => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) >> 8,
            SampleKind::Float(4) => to_i16(f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))),
            SampleKind::Float(_) => to_i16(f64::from_le_bytes([
                b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
            ])),
        }
    }
}

fn to_i16(x: f64) -> i32 {
    if x.is_nan() {
        return 0;
    }
    (x.clamp(-1.0, 1.0) * 32767.0).round() as i32
}

/// The sample kind, channels and rate of a `fmt ` chunk body.
pub(crate) fn describe(body: &[u8]) -> Option<(SampleKind, usize, usize)> {
    let u16_at = |i: usize| Some(u16::from_le_bytes([*body.get(i)?, *body.get(i + 1)?]));
    let mut tag = u16_at(0)?;
    let channels = usize::from(u16_at(2)?);
    let rate = u32::from_le_bytes([*body.get(4)?, *body.get(5)?, *body.get(6)?, *body.get(7)?]);
    let bits = u16_at(14)?;
    if tag == 0xFFFE {
        // WAVE_FORMAT_EXTENSIBLE: the real tag opens the SubFormat GUID.
        tag = u16_at(24)?;
    }
    let kind = match (tag, bits) {
        (1, 8) => SampleKind::U8,
        (1, 16) => SampleKind::Int(2),
        (1, 24) => SampleKind::Int(3),
        (1, 32) => SampleKind::Int(4),
        (3, 32) => SampleKind::Float(4),
        (3, 64) => SampleKind::Float(8),
        _ => return None,
    };
    (channels > 0 && rate > 0).then_some((kind, channels, rate as usize))
}

/// A WAV file's audio, read in blocks.
pub(crate) struct PcmReader {
    reader: BufReader<File>,
    kind: SampleKind,
    channels: usize,
    sample_rate: usize,
    /// Bytes of audio not yet read.
    remaining: u64,
    #[cfg_attr(not(feature = "flac"), allow(dead_code))]
    frames: usize,
    bytes: Vec<u8>,
}

impl PcmReader {
    /// Opens `path` and finds its audio; `format` names the format being
    /// written, for the message when the samples cannot be read.
    pub(crate) fn open(path: &Path, format: &str) -> Result<Self, ExportError> {
        let bad = |why: &str| ExportError::BadWav {
            path: path.to_owned(),
            reason: why.to_owned(),
        };
        let io = |e| ExportError::io(path, e);
        let file = File::open(path).map_err(io)?;
        let file_len = file.metadata().map_err(io)?.len();
        let mut r = BufReader::new(file);
        let mut head = [0u8; 12];
        r.read_exact(&mut head).map_err(io)?;
        if &head[..4] != b"RIFF" || &head[8..12] != b"WAVE" {
            return Err(bad("not a RIFF/WAVE file"));
        }
        let mut pos = 12u64;
        let mut fmt: Option<Vec<u8>> = None;
        loop {
            let mut ch = [0u8; 8];
            r.read_exact(&mut ch).map_err(|_| bad("no data chunk"))?;
            let size = u64::from(u32::from_le_bytes([ch[4], ch[5], ch[6], ch[7]]));
            pos += 8;
            if &ch[..4] == b"data" {
                let body = fmt.ok_or_else(|| bad("data before fmt"))?;
                let (kind, channels, rate) = describe(&body).ok_or_else(|| {
                    bad(&format!("its sample format cannot be written as {format}"))
                })?;
                let available = file_len.saturating_sub(pos);
                let len = if size == 0 || size == u64::from(u32::MAX) {
                    available
                } else {
                    size.min(available)
                };
                let frame = (kind.bytes() * channels) as u64;
                let len = len - len % frame;
                return Ok(PcmReader {
                    reader: r,
                    kind,
                    channels,
                    sample_rate: rate,
                    remaining: len,
                    frames: usize::try_from(len / frame).unwrap_or(usize::MAX),
                    bytes: Vec::new(),
                });
            }
            if &ch[..4] == b"fmt " {
                let mut body = vec![0u8; usize::try_from(size).map_err(|_| bad("huge chunk"))?];
                r.read_exact(&mut body)
                    .map_err(|_| bad("truncated chunk"))?;
                fmt = Some(body);
                if size & 1 == 1 {
                    r.seek(SeekFrom::Current(1)).map_err(io)?;
                }
            } else {
                let skip = size + (size & 1);
                r.seek(SeekFrom::Current(i64::try_from(skip).unwrap_or(i64::MAX)))
                    .map_err(io)?;
            }
            pos += size + (size & 1);
        }
    }

    /// Channels.
    pub(crate) fn channels(&self) -> usize {
        self.channels
    }

    /// Samples per second.
    pub(crate) fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    /// Bits per sample of what [`read`](Self::read) gives.
    pub(crate) fn bits(&self) -> usize {
        self.kind.bits()
    }

    /// Frames in the file (the FLAC encoder's length hint).
    #[cfg_attr(not(feature = "flac"), allow(dead_code))]
    pub(crate) fn frames(&self) -> usize {
        self.frames
    }

    /// Reads up to `max_frames` frames into `out` (interleaved, replacing
    /// its contents); returns the frames read, 0 at the end.
    pub(crate) fn read(&mut self, max_frames: usize, out: &mut Vec<i32>) -> std::io::Result<usize> {
        let frame = self.kind.bytes() * self.channels;
        let want = (max_frames * frame).min(READ_LIMIT - READ_LIMIT % frame);
        let n = usize::try_from(self.remaining).map_or(want, |r| r.min(want));
        self.bytes.resize(n, 0);
        self.reader.read_exact(&mut self.bytes)?;
        self.remaining -= n as u64;
        let kind = self.kind;
        out.clear();
        out.extend(
            self.bytes
                .chunks_exact(kind.bytes())
                .map(|b| kind.convert(b)),
        );
        Ok(n / frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_formats_convert() {
        assert_eq!(SampleKind::U8.convert(&[0x80]), 0);
        assert_eq!(SampleKind::Int(2).convert(&[0xFF, 0xFF]), -1);
        assert_eq!(SampleKind::Int(3).convert(&[0x00, 0x00, 0x80]), -8_388_608);
        assert_eq!(SampleKind::Int(4).convert(&[0, 0, 0, 0x40]), 0x0040_0000);
        assert_eq!(SampleKind::Float(4).convert(&1.0f32.to_le_bytes()), 32767);
        assert_eq!(
            SampleKind::Float(8).convert(&(-2.0f64).to_le_bytes()),
            -32767
        );
        let mut ext = vec![0u8; 40];
        ext[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
        ext[2] = 1;
        ext[4..8].copy_from_slice(&22050u32.to_le_bytes());
        ext[14] = 32;
        ext[24] = 3;
        assert_eq!(describe(&ext), Some((SampleKind::Float(4), 1, 22050)));
        ext[24] = 9;
        assert_eq!(describe(&ext), None);
    }
}
