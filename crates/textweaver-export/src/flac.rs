//! FLAC written in process, in pure Rust (`flacenc`), with the title and
//! chapters as Vorbis comments.
//!
//! The joined WAV is read back in blocks and encoded; `flacenc` keeps the
//! encoded stream in memory (about half the WAV's size for speech) and
//! writes no tags itself, so the Vorbis comment block (FLAC metadata block
//! type 4) is built here ([`crate::vorbis`]: the title, author, and
//! chapters) and added before the stream is written.
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
pub use crate::vorbis::{chapter_time, comments, vorbis_comment_block};

/// FLAC's metadata block type for Vorbis comments.
const VORBIS_COMMENT: u8 = 4;

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
