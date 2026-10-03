//! Ogg Opus written in process with libopus 1.6.1, the reference encoder
//! (`opusic-c`, built from bundled source and linked statically), with the
//! title and chapters as Vorbis comments (the `opus` feature; ADR-0011).
//!
//! Opus is the smallest of the formats for a single voice: mono at 32
//! kbit/s is about 14 MB an hour. The file follows RFC 7845:
//!
//! - **Rate.** The engine's audio is resampled (with `rubato`) to 48 kHz,
//!   Opus's own rate, unless it already is; the `OpusHead` header records
//!   the engine's rate, so decoders that honor it can give it back.
//! - **Speech settings.** One channel (stereo engines are mixed down), the
//!   Audio application, variable bit rate at [`BITRATE`], and 20 ms
//!   packets. The Audio application measured closer to the engine's
//!   waveform than VoIP on recorded speech (12.5 against 7.4 dB of
//!   signal-to-noise at 32 kbit/s), at the same size.
//! - **Exact length.** The encoder's look-ahead goes in the header as the
//!   pre-skip for players to drop at the start, and the last page's granule
//!   position trims the padding at the end, so a decoder gives back exactly
//!   as many samples (at 48 kHz) as the engine's audio lasts.
//! - **Tags.** The `OpusTags` header carries the same Vorbis comments as a
//!   FLAC file ([`crate::vorbis`]): title, author, and one `CHAPTERnnn`
//!   pair per chapter.
//!
//! The Ogg pages are written here (about one second of audio per page,
//! each with its CRC), so no Ogg crate is needed.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use opusic_c::{Application, Bitrate, Channels, Encoder, SampleRate};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Indexing, Resampler};

use crate::ExportError;
use crate::pcm::PcmReader;

/// Opus's rate, which everything is encoded at, in Hz.
pub const RATE: usize = 48_000;

/// The bit rate, in bits a second: clear for one voice.
pub const BITRATE: u32 = 32_000;

/// Samples per 20 ms packet at 48 kHz (also the unit of Ogg Opus granule
/// positions).
pub const FRAME: usize = 960;

/// Audio packets per Ogg page: about one second.
const PAGE_PACKETS: usize = 50;

/// Room for one packet (RFC 6716's largest is 1,275 bytes).
const MAX_PACKET: usize = 1500;

/// Frames read from the WAV at once.
const BLOCK: usize = 8192;

/// The `OpusHead` identification header for mono audio whose source rate
/// was `input_rate` Hz, with `pre_skip` samples (at 48 kHz) to drop at the
/// start.
pub fn opus_head(input_rate: u32, pre_skip: u16) -> Vec<u8> {
    let mut h = Vec::with_capacity(19);
    h.extend_from_slice(b"OpusHead");
    h.push(1); // version
    h.push(1); // channels
    h.extend_from_slice(&pre_skip.to_le_bytes());
    h.extend_from_slice(&input_rate.to_le_bytes());
    h.extend_from_slice(&0i16.to_le_bytes()); // output gain
    h.push(0); // channel mapping family 0: mono or stereo
    h
}

/// The `OpusTags` comment header: the magic, then a Vorbis comment body.
pub fn opus_tags(vendor: &str, comments: &[(String, String)]) -> Vec<u8> {
    let mut t = b"OpusTags".to_vec();
    t.extend(crate::vorbis::vorbis_comment_block(vendor, comments));
    t
}

/// Ogg's CRC-32 (polynomial 0x04C11DB7, not reflected, starting at 0).
fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut r = (i as u32) << 24;
        for _ in 0..8 {
            r = if r & 0x8000_0000 != 0 {
                (r << 1) ^ 0x04C1_1DB7
            } else {
                r << 1
            };
        }
        *slot = r;
    }
    table
}

/// The CRC of an Ogg page (whose CRC field is zero).
pub fn ogg_crc(page: &[u8]) -> u32 {
    use std::sync::OnceLock;
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    let table = TABLE.get_or_init(crc_table);
    page.iter().fold(0u32, |crc, &b| {
        (crc << 8) ^ table[usize::from(((crc >> 24) as u8) ^ b)]
    })
}

/// One logical Ogg stream, written page by page.
struct OggWriter<W: Write> {
    out: W,
    serial: u32,
    sequence: u32,
    /// Lacing values not yet written, each with the granule position of
    /// the packet it ends (`None` when the packet goes on).
    lacing: Vec<(u8, Option<i64>)>,
    /// The packet bytes those lacing values cover.
    body: Vec<u8>,
    /// Packets begun since the last page.
    packets: usize,
    /// The next page starts inside a packet.
    continued: bool,
}

impl<W: Write> OggWriter<W> {
    fn new(out: W, serial: u32) -> Self {
        OggWriter {
            out,
            serial,
            sequence: 0,
            lacing: Vec::new(),
            body: Vec::new(),
            packets: 0,
            continued: false,
        }
    }

    /// Adds a packet that ends at `granule`.
    fn packet(&mut self, data: &[u8], granule: i64) {
        let mut left = data.len();
        while left >= 255 {
            self.lacing.push((255, None));
            left -= 255;
        }
        self.lacing.push((left as u8, Some(granule)));
        self.body.extend_from_slice(data);
        self.packets += 1;
    }

    /// Writes one page of the first `count` lacing values.
    fn page(&mut self, count: usize, last: bool) -> std::io::Result<()> {
        let segs: Vec<(u8, Option<i64>)> = self.lacing.drain(..count).collect();
        let len: usize = segs.iter().map(|s| usize::from(s.0)).sum();
        let granule = segs.iter().rev().find_map(|s| s.1).unwrap_or(-1);
        let mut flags = 0u8;
        if self.continued {
            flags |= 0x01;
        }
        if self.sequence == 0 {
            flags |= 0x02;
        }
        if last {
            flags |= 0x04;
        }
        let mut page = Vec::with_capacity(27 + segs.len() + len);
        page.extend_from_slice(b"OggS");
        page.push(0);
        page.push(flags);
        page.extend_from_slice(&granule.to_le_bytes());
        page.extend_from_slice(&self.serial.to_le_bytes());
        page.extend_from_slice(&self.sequence.to_le_bytes());
        page.extend_from_slice(&[0; 4]);
        page.push(segs.len() as u8);
        page.extend(segs.iter().map(|s| s.0));
        page.extend(self.body.drain(..len));
        let crc = ogg_crc(&page);
        page[22..26].copy_from_slice(&crc.to_le_bytes());
        self.out.write_all(&page)?;
        self.sequence += 1;
        self.continued = segs.last().is_some_and(|s| s.1.is_none());
        Ok(())
    }

    /// Writes everything added so far, in pages of at most 255 lacing
    /// values; the last page ends the stream when `last`.
    fn flush(&mut self, last: bool) -> std::io::Result<()> {
        while self.lacing.len() > 255 {
            self.page(255, false)?;
        }
        if !self.lacing.is_empty() || last {
            self.page(self.lacing.len(), last)?;
        }
        self.packets = 0;
        Ok(())
    }

    /// Writes a page when about a second of packets is waiting.
    fn maybe_flush(&mut self) -> std::io::Result<()> {
        if self.packets >= PAGE_PACKETS || self.lacing.len() > 240 {
            self.flush(false)?;
        }
        Ok(())
    }

    fn finish(mut self) -> std::io::Result<W> {
        self.flush(true)?;
        self.out.flush()?;
        Ok(self.out)
    }
}

/// Resamples mono audio that arrives in blocks, keeping the filter's
/// state, and gives exactly `ratio * input` samples in all.
struct Rerate {
    /// `None` when the rates match.
    inner: Option<Fft<f32>>,
    pending: Vec<f32>,
    /// Output still to drop: the filter's delay.
    skip: usize,
    fed: usize,
    produced: usize,
}

impl Rerate {
    fn new(from: usize, to: usize) -> Result<Self, String> {
        let inner = if from == to {
            None
        } else {
            Some(
                Fft::<f32>::new(from, to, 1024, 1, FixedSync::Input)
                    .map_err(|e| format!("cannot resample {from} Hz audio: {e}"))?,
            )
        };
        let skip = inner.as_ref().map_or(0, |r| r.output_delay());
        Ok(Rerate {
            inner,
            pending: Vec::new(),
            skip,
            fed: 0,
            produced: 0,
        })
    }

    fn chunk(r: &mut Fft<f32>, input: &[f32], partial: Option<usize>) -> Result<Vec<f32>, String> {
        let err = |e: &dyn std::fmt::Display| format!("cannot resample: {e}");
        let frames = r.output_frames_next();
        let mut buf = vec![0.0f32; frames];
        let inp = InterleavedSlice::new(input, 1, input.len()).map_err(|e| err(&e))?;
        let mut outp = InterleavedSlice::new_mut(&mut buf, 1, frames).map_err(|e| err(&e))?;
        let indexing = partial.map(|p| Indexing::new().partial_len(p));
        let (_, produced) = r
            .process_into_buffer(&inp, &mut outp, indexing.as_ref())
            .map_err(|e| err(&e))?;
        buf.truncate(produced);
        Ok(buf)
    }

    fn keep(&mut self, mut out: Vec<f32>, limit: Option<usize>) -> Vec<f32> {
        let delay = self.skip.min(out.len());
        out.drain(..delay);
        self.skip -= delay;
        if let Some(limit) = limit {
            out.truncate(limit.saturating_sub(self.produced));
        }
        self.produced += out.len();
        out
    }

    fn push(&mut self, samples: &[f32], out: &mut Vec<f32>) -> Result<(), String> {
        self.fed += samples.len();
        let Some(r) = self.inner.as_mut() else {
            self.produced += samples.len();
            out.extend_from_slice(samples);
            return Ok(());
        };
        self.pending.extend_from_slice(samples);
        let mut got = Vec::new();
        let mut used = 0;
        loop {
            let n = r.input_frames_next();
            if self.pending.len() - used < n {
                break;
            }
            got.extend(Self::chunk(r, &self.pending[used..used + n], None)?);
            used += n;
        }
        self.pending.drain(..used);
        out.extend(self.keep(got, None));
        Ok(())
    }

    fn finish(&mut self, out: &mut Vec<f32>) -> Result<(), String> {
        let Some(r) = self.inner.as_mut() else {
            return Ok(());
        };
        let expected = (r.resample_ratio() * self.fed as f64).round() as usize;
        let n = r.input_frames_next();
        let mut left = std::mem::take(&mut self.pending);
        let partial = left.len();
        left.resize(n, 0.0);
        let mut got = Self::chunk(r, &left, Some(partial))?;
        // Silence is pumped through until the delayed output is all out.
        while self.produced + got.len().saturating_sub(self.skip) < expected {
            let more = Self::chunk(r, &left, Some(0))?;
            if more.is_empty() {
                break;
            }
            got.extend(more);
        }
        out.extend(self.keep(got, Some(expected)));
        Ok(())
    }
}

/// A serial number for the stream, different from file to file.
fn serial() -> u32 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    h.finish() as u32
}

/// Encodes the WAV at `wav` as Ogg Opus at `out`, with `comments` (see
/// [`crate::vorbis::comments`]) in its `OpusTags` header.
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
    let mut pcm = PcmReader::open(wav, "Opus")?;
    let fail = |what: String| ExportError::Opus(what);
    let codec = |e: opusic_c::ErrorCode| ExportError::Opus(e.message().to_owned());
    let channels = pcm.channels();
    let source_rate = pcm.sample_rate();
    let input_rate = u32::try_from(source_rate).map_err(|_| fail("sample rate".into()))?;
    let mut enc =
        Encoder::new(Channels::Mono, SampleRate::Hz48000, Application::Audio).map_err(codec)?;
    enc.set_bitrate(Bitrate::Value(BITRATE)).map_err(codec)?;
    enc.set_vbr(true).map_err(codec)?;
    enc.set_complexity(10).map_err(codec)?;
    let lookahead = enc.get_look_ahead().map_err(codec)? as usize;
    let pre_skip = u16::try_from(lookahead).map_err(|_| fail("look-ahead".into()))?;
    let mut rerate = Rerate::new(source_rate, RATE).map_err(fail)?;
    let scale = 1.0 / (1u32 << (pcm.bits() - 1)) as f32;

    let io = |e| ExportError::io(out, e);
    let file = BufWriter::new(File::create(out).map_err(io)?);
    let mut ogg = OggWriter::new(file, serial());
    ogg.packet(&opus_head(input_rate, pre_skip), 0);
    ogg.flush(false).map_err(io)?;
    let vendor = format!(
        "textweaver {} with {}",
        env!("CARGO_PKG_VERSION"),
        opusic_c::version()
    );
    ogg.packet(&opus_tags(&vendor, comments), 0);
    ogg.flush(false).map_err(io)?;

    let mut block: Vec<i32> = Vec::new();
    let mut mono: Vec<f32> = Vec::new();
    let mut ready: Vec<f32> = Vec::new();
    let mut packet = vec![0u8; MAX_PACKET];
    let mut granule: i64 = 0;
    // The packet just encoded is held back, so the last one can carry the
    // end-trimming granule position.
    let mut held: Option<Vec<u8>> = None;
    let mut encode_ready =
        |ready: &mut Vec<f32>, ogg: &mut OggWriter<BufWriter<File>>, held: &mut Option<Vec<u8>>| {
            let mut used = 0;
            while ready.len() - used >= FRAME {
                let n = enc
                    .encode_float_to_slice(&ready[used..used + FRAME], &mut packet)
                    .map_err(codec)?;
                used += FRAME;
                if let Some(p) = held.replace(packet[..n].to_vec()) {
                    granule += FRAME as i64;
                    ogg.packet(&p, granule);
                    ogg.maybe_flush().map_err(io)?;
                }
            }
            ready.drain(..used);
            Ok::<(), ExportError>(())
        };
    loop {
        if stop() {
            return Err(ExportError::Cancelled);
        }
        let n = pcm.read(BLOCK, &mut block).map_err(io)?;
        if n == 0 {
            break;
        }
        mono.clear();
        mono.extend(
            block
                .chunks_exact(channels)
                .map(|f| f.iter().map(|&s| s as f32).sum::<f32>() * scale / channels as f32),
        );
        rerate.push(&mono, &mut ready).map_err(fail)?;
        encode_ready(&mut ready, &mut ogg, &mut held)?;
    }
    rerate.finish(&mut ready).map_err(fail)?;
    let total = rerate.produced;
    // Silence after the end flushes the encoder's look-ahead, in whole
    // packets.
    let encoded = total - ready.len();
    let needed = total + lookahead - encoded;
    ready.resize(needed.div_ceil(FRAME) * FRAME, 0.0);
    encode_ready(&mut ready, &mut ogg, &mut held)?;
    let end = (lookahead + total) as i64;
    if let Some(p) = held.take() {
        granule += FRAME as i64;
        ogg.packet(&p, end.min(granule));
    }
    ogg.finish().map_err(io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_head_follows_rfc_7845() {
        let h = opus_head(22_050, 312);
        assert_eq!(&h[..8], b"OpusHead");
        assert_eq!(h.len(), 19);
        assert_eq!(h[8], 1);
        assert_eq!(h[9], 1);
        assert_eq!(u16::from_le_bytes([h[10], h[11]]), 312);
        assert_eq!(u32::from_le_bytes([h[12], h[13], h[14], h[15]]), 22_050);
        assert_eq!(&h[16..], [0, 0, 0]);
    }

    #[test]
    fn the_crc_matches_ogg() {
        // A single 1 bit gives the polynomial itself (libogg's table).
        assert_eq!(ogg_crc(b""), 0);
        assert_eq!(ogg_crc(&[0x01]), 0x04C1_1DB7);
    }

    #[test]
    fn long_packets_span_pages() {
        let mut w = OggWriter::new(Vec::new(), 7);
        let big = vec![9u8; 255 * 300];
        w.packet(&big, 0);
        w.flush(false).unwrap();
        w.packet(&[1, 2, 3], 960);
        let bytes = w.finish().unwrap();
        // Three pages: 255 full segments; the rest of the packet (45
        // segments of 255 and its closing 0) with the continued flag; the
        // small packet, last.
        let mut pages = Vec::new();
        let mut at = 0;
        while at < bytes.len() {
            assert_eq!(&bytes[at..at + 4], b"OggS");
            let flags = bytes[at + 5];
            let granule = i64::from_le_bytes(bytes[at + 6..at + 14].try_into().unwrap());
            let segs = usize::from(bytes[at + 26]);
            let len: usize = bytes[at + 27..at + 27 + segs]
                .iter()
                .map(|&b| usize::from(b))
                .sum();
            let mut page = bytes[at..at + 27 + segs + len].to_vec();
            let crc = u32::from_le_bytes(page[22..26].try_into().unwrap());
            page[22..26].copy_from_slice(&[0; 4]);
            assert_eq!(ogg_crc(&page), crc);
            pages.push((flags, granule, segs));
            at += 27 + segs + len;
        }
        assert_eq!(pages, [(0x02, -1, 255), (0x01, 0, 46), (0x04, 960, 1)]);
    }
}
