//! MP3 written in process with LAME (`mp3lame-encoder`), with the title
//! and chapters as an ID3v2 tag (the `mp3` feature; ADR-0011).
//!
//! LAME 3.100 is compiled from the source bundled in `mp3lame-sys` and
//! linked statically: with the `cc` crate on Windows, with its own
//! `configure` and `make` elsewhere. It is LGPL (see
//! `THIRD-PARTY-NOTICES.md` for what that asks of a distributor).
//!
//! The encoding suits speech: variable bit rate, quality 5 (LAME's `-V 5`,
//! about 130 kbit/s for music, much less for a single voice), mono for mono
//! engines, the sample rate the engine wrote. A LAME tag (Xing) in the first
//! frame gives players the exact length and the encoder's delay and
//! padding, so decoders that honor it play gaplessly and report the length
//! exactly. The ID3v2 tag ([`crate::id3tags`]) is written in front of the
//! audio afterwards.

use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use mp3lame_encoder::{Builder, FlushGap, InterleavedPcm, MonoPcm, Quality, VbrMode};

use crate::ExportError;
use crate::pcm::PcmReader;

/// Frames encoded at once.
const BLOCK: usize = 8192;

/// Encodes the WAV at `wav` as MP3 at `out` (no tags; see
/// [`crate::id3tags::write`]).
pub fn encode(wav: &Path, out: &Path) -> Result<(), ExportError> {
    let mut pcm = PcmReader::open(wav, "MP3")?;
    let fail = |what: String| ExportError::Mp3(what);
    let channels = pcm.channels();
    if channels > 2 {
        return Err(fail(format!("{channels} channels; MP3 holds one or two")));
    }
    let rate = u32::try_from(pcm.sample_rate()).map_err(|_| fail("sample rate".into()))?;
    let mut builder = Builder::new().ok_or_else(|| fail("LAME could not start".into()))?;
    builder
        .set_num_channels(channels as u8)
        .map_err(|e| fail(e.to_string()))?;
    builder
        .set_sample_rate(rate)
        .map_err(|e| fail(e.to_string()))?;
    builder
        .set_vbr_mode(VbrMode::Mtrh)
        .map_err(|e| fail(e.to_string()))?;
    builder
        .set_vbr_quality(Quality::Good)
        .map_err(|e| fail(e.to_string()))?;
    builder
        .set_quality(Quality::Good)
        .map_err(|e| fail(e.to_string()))?;
    builder
        .set_to_write_vbr_tag(true)
        .map_err(|e| fail(e.to_string()))?;
    let mut enc = builder.build().map_err(|e| fail(e.to_string()))?;

    let io = |e| ExportError::io(out, e);
    let mut file = BufWriter::new(File::create(out).map_err(io)?);
    let shift = pcm.bits().saturating_sub(16);
    let lift = 16usize.saturating_sub(pcm.bits());
    let mut block: Vec<i32> = Vec::new();
    let mut samples: Vec<i16> = Vec::new();
    let mut mp3: Vec<u8> = Vec::new();
    loop {
        let n = pcm.read(BLOCK, &mut block).map_err(io)?;
        if n == 0 {
            break;
        }
        samples.clear();
        samples.extend(block.iter().map(|&s| ((s >> shift) << lift) as i16));
        mp3.clear();
        mp3.reserve(mp3lame_encoder::max_required_buffer_size(n));
        let written = if channels == 1 {
            enc.encode_to_vec(MonoPcm(&samples), &mut mp3)
        } else {
            enc.encode_to_vec(InterleavedPcm(&samples), &mut mp3)
        };
        written.map_err(|e| fail(e.to_string()))?;
        file.write_all(&mp3).map_err(io)?;
    }
    mp3.clear();
    mp3.reserve(7200);
    enc.flush_to_vec::<FlushGap>(&mut mp3)
        .map_err(|e| fail(e.to_string()))?;
    file.write_all(&mp3).map_err(io)?;
    // The LAME tag replaces the empty first frame LAME left for it.
    mp3.clear();
    mp3.reserve(enc.lame_tag_size());
    if enc.lame_tag_encode_to_vec(&mut mp3).is_some() {
        file.seek(SeekFrom::Start(0)).map_err(io)?;
        file.write_all(&mp3).map_err(io)?;
    }
    file.flush().map_err(io)
}
