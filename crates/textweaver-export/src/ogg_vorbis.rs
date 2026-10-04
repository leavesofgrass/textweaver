//! Ogg Vorbis written in process with libvorbis and libogg (`vorbis_rs`,
//! built from bundled source and linked statically), with the title and
//! chapters as Vorbis comments (the `vorbis` feature).
//!
//! - **Signal.** The joined WAV keeps its own sample rate and channels:
//!   Vorbis takes any rate, so nothing is resampled or mixed down.
//! - **Quality.** Quality-based VBR at 0.3 on libvorbis's -0.1 to 1.0
//!   scale, the same as `oggenc`'s default quality 3: clear speech in a
//!   small file (about 40 to 60 kbit/s for mono speech). textweaver has no
//!   quality setting for the other formats, so this one has none either.
//! - **Tags.** The comment header carries the same Vorbis comments as a
//!   FLAC or Opus file ([`crate::vorbis`]): title, author, and one
//!   `CHAPTERnnn` pair per chapter.

use std::fs::File;
use std::io::BufWriter;
use std::num::{NonZeroU8, NonZeroU32};
use std::path::Path;

use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};

use crate::ExportError;
use crate::pcm::PcmReader;

/// The encoder's quality (libvorbis's scale, -0.1 to 1.0; `oggenc -q 3`).
pub const QUALITY: f32 = 0.3;

/// Frames read and encoded at once (libvorbis suggests about 1024; a few
/// times that keeps the stop check frequent and the calls few).
const BLOCK: usize = 4096;

/// A serial number for the stream, different from file to file.
fn serial() -> i32 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    h.finish() as i32
}

/// Encodes the WAV at `wav` as Ogg Vorbis at `out`, with `comments` (see
/// [`crate::vorbis::comments`]) in its comment header.
pub fn encode(wav: &Path, out: &Path, comments: &[(String, String)]) -> Result<(), ExportError> {
    encode_with_stop(wav, out, comments, &|| false)
}

/// [`encode`], asking `stop` before each block: when it says stop, the
/// encoding ends in [`ExportError::Cancelled`] (the caller removes the
/// partial file).
pub fn encode_with_stop(
    wav: &Path,
    out: &Path,
    comments: &[(String, String)],
    stop: &dyn Fn() -> bool,
) -> Result<(), ExportError> {
    let mut pcm = PcmReader::open(wav, "Ogg Vorbis")?;
    let codec = |e: vorbis_rs::VorbisError| ExportError::Vorbis(e.to_string());
    let fail = |what: &str| ExportError::Vorbis(format!("the {what} is not supported"));
    let channels = pcm.channels();
    let rate = u32::try_from(pcm.sample_rate())
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or_else(|| fail("sample rate"))?;
    let count = u8::try_from(channels)
        .ok()
        .and_then(NonZeroU8::new)
        .ok_or_else(|| fail("channel count"))?;
    let scale = 1.0 / (1u32 << (pcm.bits() - 1)) as f32;

    let io = |e| ExportError::io(out, e);
    let file = BufWriter::new(File::create(out).map_err(io)?);
    let mut builder = VorbisEncoderBuilder::new_with_serial(rate, count, file, serial());
    builder.bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
        target_quality: QUALITY,
    });
    builder
        .comment_tags(comments.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .map_err(codec)?;
    let mut enc = builder.build().map_err(codec)?;

    let mut block: Vec<i32> = Vec::new();
    let mut planes: Vec<Vec<f32>> = vec![Vec::with_capacity(BLOCK); channels];
    loop {
        if stop() {
            return Err(ExportError::Cancelled);
        }
        let n = pcm.read(BLOCK, &mut block).map_err(io)?;
        if n == 0 {
            break;
        }
        for (c, plane) in planes.iter_mut().enumerate() {
            plane.clear();
            plane.extend(
                block
                    .iter()
                    .skip(c)
                    .step_by(channels)
                    .map(|&s| s as f32 * scale),
            );
        }
        enc.encode_audio_block(&planes).map_err(codec)?;
    }
    let mut file = enc.finish().map_err(codec)?;
    std::io::Write::flush(&mut file).map_err(io)?;
    Ok(())
}
