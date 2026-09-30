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

use std::path::Path;

use flacenc::component::{BitRepr, MetadataBlockData};
use flacenc::error::{SourceError, Verify};
use flacenc::source::{Fill, Source};

use crate::ExportError;
use crate::pcm::PcmReader;
use crate::timeline::Chapter;

/// FLAC's metadata block type for Vorbis comments.
const VORBIS_COMMENT: u8 = 4;

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

/// The WAV's audio as a FLAC source.
struct WavSource {
    pcm: PcmReader,
    samples: Vec<i32>,
}

impl Source for WavSource {
    fn channels(&self) -> usize {
        self.pcm.channels()
    }

    fn bits_per_sample(&self) -> usize {
        self.pcm.bits()
    }

    fn sample_rate(&self) -> usize {
        self.pcm.sample_rate()
    }

    fn read_samples<F: Fill>(
        &mut self,
        block_size: usize,
        dest: &mut F,
    ) -> Result<usize, SourceError> {
        let n = self
            .pcm
            .read(block_size, &mut self.samples)
            .map_err(SourceError::from_io_error)?;
        dest.fill_interleaved(&self.samples)?;
        Ok(n)
    }

    fn len_hint(&self) -> Option<usize> {
        Some(self.pcm.frames())
    }
}

/// Encodes the WAV at `wav` as FLAC at `out`, with `comments` as its Vorbis
/// comment block.
pub fn encode(wav: &Path, out: &Path, comments: &[(String, String)]) -> Result<(), ExportError> {
    let source = WavSource {
        pcm: PcmReader::open(wav, "FLAC")?,
        samples: Vec::new(),
    };
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
}
