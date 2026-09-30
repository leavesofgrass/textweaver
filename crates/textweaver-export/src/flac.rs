//! FLAC written in process, in pure Rust (`flacenc`), with the title and
//! chapters as Vorbis comments.
//!
//! The joined WAV is read back in blocks and encoded; `flacenc` keeps the
//! encoded stream in memory (about half the WAV's size for speech) and
//! writes no tags itself, so the Vorbis comment block (FLAC metadata block
//! type 4) is built here and added before the stream is written:
//!
//! - `TITLE` and `ALBUM`: the document's title; `ARTIST`: its author;
//!   `GENRE=Audiobook` (the same tags the M4B gets through ffmpeg);
//! - one `CHAPTERnnn=HH:MM:SS.mmm` and `CHAPTERnnnNAME=title` pair per
//!   chapter, the chapter convention audiobook players read from Vorbis
//!   comments.
//!
//! Sample formats: 8, 16, 24 and 32-bit PCM and 32 and 64-bit float, the
//! formats engines write. 32-bit PCM keeps its top 24 bits and float
//! becomes 16-bit PCM, because FLAC stores integers of at most 24 bits
//! (as `flacenc` supports them).

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use flacenc::component::{BitRepr, MetadataBlockData};
use flacenc::error::{SourceError, Verify};
use flacenc::source::{Fill, Source};

use crate::ExportError;
use crate::timeline::Chapter;

/// FLAC's metadata block type for Vorbis comments.
const VORBIS_COMMENT: u8 = 4;

/// Bytes read from the WAV per block, at most (the encoder asks for one
/// block of samples at a time).
const READ_LIMIT: usize = 1 << 20;

/// `HH:MM:SS.mmm` for a time in milliseconds (the chapter time format).
pub fn chapter_time(ms: u64) -> String {
    let (h, rest) = (ms / 3_600_000, ms % 3_600_000);
    let (m, rest) = (rest / 60_000, rest % 60_000);
    let (s, milli) = (rest / 1000, rest % 1000);
    format!("{h:02}:{m:02}:{s:02}.{milli:03}")
}

/// The Vorbis comments for an export: tags, then the chapters in order.
pub fn comments(
    title: Option<&str>,
    author: Option<&str>,
    chapters: &[Chapter],
) -> Vec<(String, String)> {
    let mut c = Vec::new();
    if let Some(t) = title.filter(|t| !t.trim().is_empty()) {
        c.push(("TITLE".to_owned(), t.to_owned()));
        c.push(("ALBUM".to_owned(), t.to_owned()));
    }
    if let Some(a) = author.filter(|a| !a.trim().is_empty()) {
        c.push(("ARTIST".to_owned(), a.to_owned()));
    }
    c.push(("GENRE".to_owned(), "Audiobook".to_owned()));
    for (i, ch) in chapters.iter().enumerate() {
        let key = format!("CHAPTER{:03}", i + 1);
        c.push((key.clone(), chapter_time(ch.start_ms)));
        c.push((format!("{key}NAME"), ch.title.clone()));
    }
    c
}

/// The body of a Vorbis comment block: the vendor string, then each
/// `KEY=value` comment, every length a 32-bit little-endian count of UTF-8
/// bytes. Newlines in values become spaces.
pub fn vorbis_comment_block(vendor: &str, comments: &[(String, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    let put = |s: &str, out: &mut Vec<u8>| {
        let len = u32::try_from(s.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&s.as_bytes()[..len as usize]);
    };
    put(vendor, &mut out);
    let count = u32::try_from(comments.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&count.to_le_bytes());
    for (k, v) in comments {
        let v = v.replace(['\r', '\n'], " ");
        put(&format!("{k}={v}"), &mut out);
    }
    out
}

/// How samples are stored in the WAV being encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SampleKind {
    /// Unsigned 8-bit PCM.
    U8,
    /// Signed little-endian PCM of 2, 3 or 4 bytes.
    Int(usize),
    /// IEEE float of 4 or 8 bytes.
    Float(usize),
}

impl SampleKind {
    fn bytes(self) -> usize {
        match self {
            SampleKind::U8 => 1,
            SampleKind::Int(n) | SampleKind::Float(n) => n,
        }
    }

    /// Bits per sample in the FLAC stream.
    fn flac_bits(self) -> usize {
        match self {
            SampleKind::U8 => 8,
            SampleKind::Int(2) | SampleKind::Float(_) => 16,
            SampleKind::Int(_) => 24,
        }
    }

    fn convert(self, b: &[u8]) -> i32 {
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

/// The WAV's audio: where its `data` chunk is and how to read it.
struct WavSource {
    reader: BufReader<File>,
    kind: SampleKind,
    channels: usize,
    sample_rate: usize,
    /// Bytes of audio not yet read.
    remaining: u64,
    frames: usize,
    bytes: Vec<u8>,
    samples: Vec<i32>,
}

impl WavSource {
    fn open(path: &Path) -> Result<Self, ExportError> {
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
                let (kind, channels, rate) = describe(&body)
                    .ok_or_else(|| bad("its sample format cannot be written as FLAC"))?;
                let available = file_len.saturating_sub(pos);
                let len = if size == 0 || size == u64::from(u32::MAX) {
                    available
                } else {
                    size.min(available)
                };
                let frame = (kind.bytes() * channels) as u64;
                let len = len - len % frame;
                return Ok(WavSource {
                    reader: r,
                    kind,
                    channels,
                    sample_rate: rate,
                    remaining: len,
                    frames: usize::try_from(len / frame).unwrap_or(usize::MAX),
                    bytes: Vec::new(),
                    samples: Vec::new(),
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
}

/// The sample kind, channels and rate of a `fmt ` chunk body.
fn describe(body: &[u8]) -> Option<(SampleKind, usize, usize)> {
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

impl Source for WavSource {
    fn channels(&self) -> usize {
        self.channels
    }

    fn bits_per_sample(&self) -> usize {
        self.kind.flac_bits()
    }

    fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    fn read_samples<F: Fill>(
        &mut self,
        block_size: usize,
        dest: &mut F,
    ) -> Result<usize, SourceError> {
        let frame = self.kind.bytes() * self.channels;
        let want = (block_size * frame).min(READ_LIMIT - READ_LIMIT % frame);
        let n = usize::try_from(self.remaining).map_or(want, |r| r.min(want));
        self.bytes.resize(n, 0);
        self.reader
            .read_exact(&mut self.bytes)
            .map_err(SourceError::from_io_error)?;
        self.remaining -= n as u64;
        let kind = self.kind;
        self.samples.clear();
        self.samples.extend(
            self.bytes
                .chunks_exact(kind.bytes())
                .map(|b| kind.convert(b)),
        );
        dest.fill_interleaved(&self.samples)?;
        Ok(n / frame)
    }

    fn len_hint(&self) -> Option<usize> {
        Some(self.frames)
    }
}

/// Encodes the WAV at `wav` as FLAC at `out`, with `comments` as its Vorbis
/// comment block.
pub fn encode(wav: &Path, out: &Path, comments: &[(String, String)]) -> Result<(), ExportError> {
    let source = WavSource::open(wav)?;
    let flac_err = |what: String| ExportError::Flac(what);
    let mut config = flacenc::config::Encoder::default();
    config.multithread = false;
    let config = config
        .into_verified()
        .map_err(|(_, e)| flac_err(e.to_string()))?;
    let block_size = config.block_size;
    let mut stream = flacenc::encode_with_fixed_block_size(&config, source, block_size)
        .map_err(|e| flac_err(e.to_string()))?;
    let vendor = format!("textweaver {}", env!("CARGO_PKG_VERSION"));
    let block = vorbis_comment_block(&vendor, comments);
    stream.add_metadata_block(
        MetadataBlockData::new_unknown(VORBIS_COMMENT, &block)
            .map_err(|e| flac_err(e.to_string()))?,
    );
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|e| flac_err(e.to_string()))?;
    std::fs::write(out, sink.as_slice()).map_err(|e| ExportError::io(out, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_times_read_as_clock_times() {
        assert_eq!(chapter_time(0), "00:00:00.000");
        assert_eq!(chapter_time(3_723_045), "01:02:03.045");
    }

    #[test]
    fn comments_carry_tags_then_chapters() {
        let ch = |title: &str, start_ms| Chapter {
            title: title.to_owned(),
            start_ms,
            end_ms: start_ms + 1000,
            source_start: textweaver_core::CharPos(0),
        };
        let c = comments(Some("Plants"), None, &[ch("Intro", 0), ch("Light", 61_500)]);
        let keys: Vec<String> = c.iter().map(|(k, v)| format!("{k}={v}")).collect();
        assert_eq!(
            keys,
            [
                "TITLE=Plants",
                "ALBUM=Plants",
                "GENRE=Audiobook",
                "CHAPTER001=00:00:00.000",
                "CHAPTER001NAME=Intro",
                "CHAPTER002=00:01:01.500",
                "CHAPTER002NAME=Light",
            ]
        );
    }

    #[test]
    fn the_comment_block_is_length_prefixed() {
        let b = vorbis_comment_block("tw", &[("TITLE".into(), "a\nb".into())]);
        assert_eq!(
            b,
            [
                2, 0, 0, 0, b't', b'w', 1, 0, 0, 0, 9, 0, 0, 0, b'T', b'I', b'T', b'L', b'E', b'=',
                b'a', b' ', b'b'
            ]
        );
    }

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
