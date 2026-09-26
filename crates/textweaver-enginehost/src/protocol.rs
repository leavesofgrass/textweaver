//! The engine-host wire protocol (ADR-0012): framing, the messages every
//! engine shares, and the building blocks engine crates use for their own.
//!
//! Every frame is `len: u32 LE`, then `len` bytes: a one-byte tag followed
//! by the tag's payload. Integers are little-endian; strings are a `u32`
//! byte length followed by UTF-8; lists are a `u32` count followed by the
//! items. Requests (backend to host, on the host's stdin) have tags
//! `0x01..=0x7f`; replies (host to backend, on its stdout) have tags
//! `0x81..=0xff`.
//!
//! Shared messages, identical for every engine ([`tag`]):
//!
//! | tag  | message | payload |
//! |------|---------|---------|
//! | 0x01 | Speak   | `token: u64`, then the engine's utterance encoding |
//! | 0x02 | Stop    | (none) |
//! | 0x05 | Quit    | (none) |
//! | 0x81 | Ready   | `protocol: u16`, `sample_rate: u32`, then engine fields |
//! | 0x82 | Audio   | `token: u64`, `count: u32`, `count` × `i16` samples |
//! | 0x83 | Word    | `token: u64`, the engine's word position, `sample: u64` last |
//! | 0x84 | End     | `token: u64`, `status: u8` ([`EndStatus`]), `samples: u64` |
//! | 0x85 | Error   | `token: u64` (0 = none), `message: str` |
//!
//! Tags `0x03`, `0x04`, `0x06..=0x7f` and `0x86..=0xff` are the engine's
//! own (voice selection, parameters, voice listing, dictionary reports).
//!
//! Conversation: the host announces itself with `Ready` (preceded only by
//! engine-specific start-up reports, or replaced by one `Error` if it cannot
//! start). Each `Speak` is answered by any number of `Audio` and `Word`
//! frames followed by exactly one `End` (preceded by `Error` when synthesis
//! failed). `Stop` aborts the utterance in progress and every `Speak`
//! received before it; each still gets its `End` ([`EndStatus::Aborted`]).
//! The host exits cleanly on `Quit` or end of input.
//!
//! Versioning: [`PROTOCOL_VERSION`] is the first field of every `Ready`,
//! and a backend refuses a host whose version differs
//! ([`check_version`]). The version changes whenever the layout of any
//! frame, shared or engine-specific, changes.

use std::io::{self, Read, Write};

/// Version of the engine-host protocol, the first field of every `Ready`.
pub const PROTOCOL_VERSION: u16 = 1;

/// Largest frame either side accepts (16 MiB). Larger lengths mean a
/// corrupt stream.
pub const MAX_FRAME: usize = 16 * 1024 * 1024;

/// Frame tags shared by every engine.
pub mod tag {
    /// Request: synthesize an utterance.
    pub const SPEAK: u8 = 0x01;
    /// Request: abort synthesis and skip every earlier `Speak`.
    pub const STOP: u8 = 0x02;
    /// Request: exit cleanly.
    pub const QUIT: u8 = 0x05;
    /// Reply: the host is ready (protocol version and sample rate first).
    pub const READY: u8 = 0x81;
    /// Reply: PCM samples for an utterance.
    pub const AUDIO: u8 = 0x82;
    /// Reply: a word position with its sample offset (engine-specific
    /// position fields).
    pub const WORD: u8 = 0x83;
    /// Reply: an utterance ended.
    pub const END: u8 = 0x84;
    /// Reply: something failed.
    pub const ERROR: u8 = 0x85;
}

/// Outcome of one `Speak`, reported in its `End` frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndStatus {
    /// All audio was delivered.
    Done,
    /// A `Stop` cut synthesis short (or skipped it).
    Aborted,
    /// The engine failed; an `Error` frame preceded this one.
    Failed,
}

impl EndStatus {
    /// The status byte on the wire.
    pub fn to_byte(self) -> u8 {
        match self {
            EndStatus::Done => 0,
            EndStatus::Aborted => 1,
            EndStatus::Failed => 2,
        }
    }

    /// Parses a status byte.
    pub fn from_byte(b: u8) -> Result<Self, ProtocolError> {
        match b {
            0 => Ok(EndStatus::Done),
            1 => Ok(EndStatus::Aborted),
            2 => Ok(EndStatus::Failed),
            other => Err(ProtocolError::BadValue("end status", u32::from(other))),
        }
    }
}

/// Protocol failures.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    /// The stream ended in the middle of a frame.
    #[error("stream ended inside a frame")]
    Truncated,
    /// A frame claimed a length over [`MAX_FRAME`] or under one byte.
    #[error("bad frame length {0}")]
    BadLength(usize),
    /// Unknown frame tag.
    #[error("unknown frame tag {0:#04x}")]
    BadTag(u8),
    /// A field held an impossible value.
    #[error("bad {0}: {1}")]
    BadValue(&'static str, u32),
    /// A string was not UTF-8.
    #[error("string is not UTF-8")]
    BadUtf8,
    /// A frame had bytes left over after its payload.
    #[error("{0} trailing bytes in frame")]
    Trailing(usize),
    /// Reading or writing the pipe failed.
    #[error("i/o: {0}")]
    Io(#[from] io::Error),
}

/// A message that travels as one frame: an engine's request or reply type.
pub trait Message: Sized {
    /// The complete frame (length prefix included).
    fn encode(&self) -> Vec<u8>;
    /// Parses one frame body (tag and payload, without the length prefix).
    fn decode(body: &[u8]) -> Result<Self, ProtocolError>;
}

/// Builds one frame: length placeholder, tag, payload.
#[derive(Debug)]
pub struct FrameWriter(Vec<u8>);

impl FrameWriter {
    /// Starts a frame with `tag`.
    pub fn new(tag: u8) -> Self {
        // Length placeholder, patched in `finish`.
        FrameWriter(vec![0, 0, 0, 0, tag])
    }
    /// Reserves room for `n` more payload bytes.
    pub fn reserve(&mut self, n: usize) {
        self.0.reserve(n);
    }
    /// Appends a `u8`.
    pub fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    /// Appends an `i8`.
    pub fn i8(&mut self, v: i8) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    /// Appends a `u16`.
    pub fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    /// Appends a `u32`.
    pub fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    /// Appends an `i32`.
    pub fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    /// Appends a `u64`.
    pub fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    /// Appends a list count or byte length as a `u32`.
    pub fn len(&mut self, n: usize) {
        self.u32(u32::try_from(n).unwrap_or(u32::MAX));
    }
    /// Appends a string: byte length, then UTF-8.
    pub fn str(&mut self, s: &str) {
        self.len(s.len());
        self.0.extend_from_slice(s.as_bytes());
    }
    /// Appends a sample list: count, then 16-bit samples.
    pub fn samples(&mut self, samples: &[i16]) {
        self.0.reserve(4 + samples.len() * 2);
        self.len(samples.len());
        for s in samples {
            self.0.extend_from_slice(&s.to_le_bytes());
        }
    }
    /// The finished frame, length prefix filled in.
    pub fn finish(mut self) -> Vec<u8> {
        let n = u32::try_from(self.0.len() - 4).unwrap_or(u32::MAX);
        self.0[..4].copy_from_slice(&n.to_le_bytes());
        self.0
    }
}

/// Reads the payload of one frame body.
#[derive(Debug)]
pub struct FrameReader<'a>(&'a [u8]);

impl<'a> FrameReader<'a> {
    /// Splits a frame body into its tag and a reader over the payload.
    pub fn open(body: &'a [u8]) -> Result<(u8, Self), ProtocolError> {
        let (&t, rest) = body.split_first().ok_or(ProtocolError::BadLength(0))?;
        Ok((t, FrameReader(rest)))
    }
    /// The next `n` bytes.
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], ProtocolError> {
        if self.0.len() < n {
            return Err(ProtocolError::Truncated);
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], ProtocolError> {
        let mut a = [0; N];
        a.copy_from_slice(self.take(N)?);
        Ok(a)
    }
    /// Reads a `u8`.
    pub fn u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.array::<1>()?[0])
    }
    /// Reads an `i8`.
    pub fn i8(&mut self) -> Result<i8, ProtocolError> {
        Ok(i8::from_le_bytes(self.array()?))
    }
    /// Reads a `u16`.
    pub fn u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    /// Reads a `u32`.
    pub fn u32(&mut self) -> Result<u32, ProtocolError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    /// Reads an `i32`.
    pub fn i32(&mut self) -> Result<i32, ProtocolError> {
        Ok(i32::from_le_bytes(self.array()?))
    }
    /// Reads a `u64`.
    pub fn u64(&mut self) -> Result<u64, ProtocolError> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    /// Reads a list count, checked against the bytes left (each item takes
    /// at least `min_item` bytes) so a corrupt count cannot trigger a huge
    /// allocation.
    pub fn count(&mut self, min_item: usize) -> Result<usize, ProtocolError> {
        let n = self.u32()? as usize;
        if n.saturating_mul(min_item.max(1)) > self.0.len() {
            return Err(ProtocolError::Truncated);
        }
        Ok(n)
    }
    /// Reads a string.
    pub fn str(&mut self) -> Result<String, ProtocolError> {
        let n = self.count(1)?;
        let bytes = self.take(n)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ProtocolError::BadUtf8)
    }
    /// Reads a sample list.
    pub fn samples(&mut self) -> Result<Vec<i16>, ProtocolError> {
        let n = self.count(2)?;
        let bytes = self.take(n * 2)?;
        Ok(bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect())
    }
    /// Checks that the payload was read completely.
    pub fn finish(self) -> Result<(), ProtocolError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ProtocolError::Trailing(self.0.len()))
        }
    }
}

/// The `Stop` frame.
pub fn encode_stop() -> Vec<u8> {
    FrameWriter::new(tag::STOP).finish()
}

/// The `Quit` frame.
pub fn encode_quit() -> Vec<u8> {
    FrameWriter::new(tag::QUIT).finish()
}

/// An `Audio` frame straight from a sample slice (the hosts' hot path).
pub fn encode_audio(token: u64, samples: &[i16]) -> Vec<u8> {
    let mut e = FrameWriter::new(tag::AUDIO);
    e.u64(token);
    e.samples(samples);
    e.finish()
}

/// An `End` frame.
pub fn encode_end(token: u64, status: EndStatus, samples: u64) -> Vec<u8> {
    let mut e = FrameWriter::new(tag::END);
    e.u64(token);
    e.u8(status.to_byte());
    e.u64(samples);
    e.finish()
}

/// An `Error` frame (`token` 0 when no utterance is involved).
pub fn encode_error(token: u64, message: &str) -> Vec<u8> {
    let mut e = FrameWriter::new(tag::ERROR);
    e.u64(token);
    e.str(message);
    e.finish()
}

/// The fields every `Ready` frame starts with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadyHeader {
    /// The host's [`PROTOCOL_VERSION`].
    pub protocol: u16,
    /// Audio sample rate in Hz (16-bit mono PCM).
    pub sample_rate: u32,
}

impl ReadyHeader {
    /// Reads the shared header of a `Ready` frame body; `None` for any
    /// other frame, so a backend can check the version of a host whose
    /// engine-specific fields it cannot parse.
    pub fn peek(body: &[u8]) -> Option<ReadyHeader> {
        let (t, mut r) = FrameReader::open(body).ok()?;
        if t != tag::READY {
            return None;
        }
        Some(ReadyHeader {
            protocol: r.u16().ok()?,
            sample_rate: r.u32().ok()?,
        })
    }

    /// Writes the header into a `Ready` frame being built.
    pub fn write(&self, e: &mut FrameWriter) {
        e.u16(self.protocol);
        e.u32(self.sample_rate);
    }
}

/// Accepts a host whose `Ready` reported `protocol`, or explains why not.
pub fn check_version(protocol: u16) -> Result<(), String> {
    if protocol == PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(format!(
            "host speaks protocol {protocol}, expected {PROTOCOL_VERSION} \
             (rebuild the hosts with `cargo xtask hosts`)"
        ))
    }
}

/// True when a frame body is exactly a `Stop` request.
pub fn is_stop(body: &[u8]) -> bool {
    body == [tag::STOP]
}

/// True when a frame body is exactly a `Quit` request.
pub fn is_quit(body: &[u8]) -> bool {
    body == [tag::QUIT]
}

/// Accumulates bytes from a pipe and yields complete frame bodies, so reads
/// of any size (including one byte at a time) reassemble correctly.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buf: Vec<u8>,
    start: usize,
}

impl FrameDecoder {
    /// An empty decoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends bytes read from the stream.
    pub fn push(&mut self, bytes: &[u8]) {
        if self.start > 0 && self.start == self.buf.len() {
            self.buf.clear();
            self.start = 0;
        }
        self.buf.extend_from_slice(bytes);
    }

    /// The next complete frame body (tag and payload), if one is buffered.
    pub fn next_body(&mut self) -> Result<Option<Vec<u8>>, ProtocolError> {
        let avail = &self.buf[self.start..];
        if avail.len() < 4 {
            return Ok(None);
        }
        let len = u32::from_le_bytes([avail[0], avail[1], avail[2], avail[3]]) as usize;
        if len == 0 || len > MAX_FRAME {
            return Err(ProtocolError::BadLength(len));
        }
        if avail.len() < 4 + len {
            return Ok(None);
        }
        let body = avail[4..4 + len].to_vec();
        self.start += 4 + len;
        if self.start > 64 * 1024 && self.start * 2 > self.buf.len() {
            self.buf.drain(..self.start);
            self.start = 0;
        }
        Ok(Some(body))
    }

    /// True when a partial frame is buffered.
    pub fn has_partial(&self) -> bool {
        self.start < self.buf.len()
    }
}

/// Reads one frame body from a blocking reader. `Ok(None)` on a clean end
/// of stream (no byte of a new frame read).
pub fn read_body(r: &mut impl Read) -> Result<Option<Vec<u8>>, ProtocolError> {
    let mut len = [0u8; 4];
    let mut got = 0;
    while got < 4 {
        match r.read(&mut len[got..]) {
            Ok(0) if got == 0 => return Ok(None),
            Ok(0) => return Err(ProtocolError::Truncated),
            Ok(n) => got += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    let len = u32::from_le_bytes(len) as usize;
    if len == 0 || len > MAX_FRAME {
        return Err(ProtocolError::BadLength(len));
    }
    let mut body = vec![0; len];
    r.read_exact(&mut body).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            ProtocolError::Truncated
        } else {
            ProtocolError::Io(e)
        }
    })?;
    Ok(Some(body))
}

/// Writes one pre-encoded frame and flushes.
pub fn write_frame(w: &mut impl Write, frame: &[u8]) -> io::Result<()> {
    w.write_all(frame)?;
    w.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames() -> Vec<Vec<u8>> {
        let mut ready = FrameWriter::new(tag::READY);
        ReadyHeader {
            protocol: PROTOCOL_VERSION,
            sample_rate: 22050,
        }
        .write(&mut ready);
        ready.str("x86");
        vec![
            ready.finish(),
            encode_audio(7, &[0, 1, -1, i16::MAX, i16::MIN]),
            encode_audio(7, &[]),
            encode_end(7, EndStatus::Aborted, 99),
            encode_error(0, "no engine"),
            encode_stop(),
            encode_quit(),
        ]
    }

    #[test]
    fn the_writer_prefixes_the_length() {
        for f in frames() {
            let len = u32::from_le_bytes([f[0], f[1], f[2], f[3]]) as usize;
            assert_eq!(len, f.len() - 4);
        }
    }

    #[test]
    fn shared_frames_read_back() {
        let f = encode_audio(7, &[0, 1, -1, i16::MAX, i16::MIN]);
        let (t, mut r) = FrameReader::open(&f[4..]).unwrap();
        assert_eq!(t, tag::AUDIO);
        assert_eq!(r.u64().unwrap(), 7);
        assert_eq!(r.samples().unwrap(), [0, 1, -1, i16::MAX, i16::MIN]);
        r.finish().unwrap();

        let f = encode_end(3, EndStatus::Failed, 12);
        let (t, mut r) = FrameReader::open(&f[4..]).unwrap();
        assert_eq!(t, tag::END);
        assert_eq!(r.u64().unwrap(), 3);
        assert_eq!(
            EndStatus::from_byte(r.u8().unwrap()).unwrap(),
            EndStatus::Failed
        );
        assert_eq!(r.u64().unwrap(), 12);
        r.finish().unwrap();

        let f = encode_error(0, "naïve — 日本");
        let (t, mut r) = FrameReader::open(&f[4..]).unwrap();
        assert_eq!(t, tag::ERROR);
        assert_eq!(r.u64().unwrap(), 0);
        assert_eq!(r.str().unwrap(), "naïve — 日本");
        r.finish().unwrap();

        assert!(is_stop(&encode_stop()[4..]));
        assert!(is_quit(&encode_quit()[4..]));
        assert!(!is_stop(&[tag::STOP, 0]));
    }

    #[test]
    fn every_status_round_trips() {
        for s in [EndStatus::Done, EndStatus::Aborted, EndStatus::Failed] {
            assert_eq!(EndStatus::from_byte(s.to_byte()).unwrap(), s);
        }
        assert!(matches!(
            EndStatus::from_byte(9),
            Err(ProtocolError::BadValue("end status", 9))
        ));
    }

    #[test]
    fn the_ready_header_is_readable_without_the_engine_fields() {
        let f = &frames()[0];
        assert_eq!(
            ReadyHeader::peek(&f[4..]),
            Some(ReadyHeader {
                protocol: PROTOCOL_VERSION,
                sample_rate: 22050
            })
        );
        assert_eq!(ReadyHeader::peek(&encode_stop()[4..]), None);
        assert_eq!(ReadyHeader::peek(&[tag::READY, 1]), None);
        assert!(check_version(PROTOCOL_VERSION).is_ok());
        assert!(check_version(PROTOCOL_VERSION + 1).is_err());
    }

    #[test]
    fn decoder_reassembles_byte_by_byte() {
        let stream: Vec<u8> = frames().concat();
        let mut dec = FrameDecoder::new();
        let mut got = Vec::new();
        for b in &stream {
            dec.push(std::slice::from_ref(b));
            while let Some(body) = dec.next_body().unwrap() {
                got.push(body);
            }
        }
        let want: Vec<Vec<u8>> = frames().iter().map(|f| f[4..].to_vec()).collect();
        assert_eq!(got, want);
        assert!(!dec.has_partial());
    }

    #[test]
    fn decoder_handles_odd_chunks() {
        let stream: Vec<u8> = frames().concat();
        let want: Vec<Vec<u8>> = frames().iter().map(|f| f[4..].to_vec()).collect();
        for chunk in [2, 3, 7, 13, 1000] {
            let mut dec = FrameDecoder::new();
            let mut got = Vec::new();
            for c in stream.chunks(chunk) {
                dec.push(c);
                while let Some(body) = dec.next_body().unwrap() {
                    got.push(body);
                }
            }
            assert_eq!(got, want, "chunk size {chunk}");
        }
    }

    /// A reader that returns at most one byte per call (a pipe under load).
    struct Trickle<'a>(&'a [u8]);

    impl Read for Trickle<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.0.is_empty() || buf.is_empty() {
                return Ok(0);
            }
            buf[0] = self.0[0];
            self.0 = &self.0[1..];
            Ok(1)
        }
    }

    #[test]
    fn blocking_reads_assemble_frames_then_end_cleanly() {
        let stream: Vec<u8> = frames().concat();
        let mut t = Trickle(&stream);
        let mut n = 0;
        while read_body(&mut t).unwrap().is_some() {
            n += 1;
        }
        assert_eq!(n, frames().len());
        let mut out = Vec::new();
        for f in frames() {
            write_frame(&mut out, &f).unwrap();
        }
        assert_eq!(out, stream);
    }

    #[test]
    fn truncated_and_corrupt_frames_are_errors() {
        let f = encode_end(1, EndStatus::Done, 0);
        assert!(matches!(
            read_body(&mut &f[..2]),
            Err(ProtocolError::Truncated)
        ));
        assert!(matches!(
            read_body(&mut &f[..f.len() - 1]),
            Err(ProtocolError::Truncated)
        ));
        assert!(matches!(
            read_body(&mut &[0u8, 0, 0, 0][..]),
            Err(ProtocolError::BadLength(0))
        ));
        assert!(matches!(
            read_body(&mut &[0xff, 0xff, 0xff, 0x7f][..]),
            Err(ProtocolError::BadLength(_))
        ));
        let mut dec = FrameDecoder::new();
        dec.push(&u32::MAX.to_le_bytes());
        assert!(matches!(dec.next_body(), Err(ProtocolError::BadLength(_))));
        assert!(matches!(
            FrameReader::open(&[]),
            Err(ProtocolError::BadLength(0))
        ));
        // A count larger than the frame is truncation, not a huge allocation.
        let huge = [tag::AUDIO, 1, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0x0f];
        let (_, mut r) = FrameReader::open(&huge).unwrap();
        r.u64().unwrap();
        assert!(matches!(r.samples(), Err(ProtocolError::Truncated)));
        let (_, mut r) = FrameReader::open(&[tag::ERROR, 2, 0, 0, 0, 0xff, 0xfe]).unwrap();
        assert!(matches!(r.str(), Err(ProtocolError::BadUtf8)));
        let (_, r) = FrameReader::open(&[tag::STOP, 0]).unwrap();
        assert!(matches!(r.finish(), Err(ProtocolError::Trailing(1))));
    }

    proptest::proptest! {
        #[test]
        fn audio_round_trips(token: u64, samples in proptest::collection::vec(proptest::num::i16::ANY, 0..300)) {
            let f = encode_audio(token, &samples);
            let mut dec = FrameDecoder::new();
            dec.push(&f);
            let body = dec.next_body().unwrap().unwrap();
            let (t, mut r) = FrameReader::open(&body).unwrap();
            proptest::prop_assert_eq!(t, tag::AUDIO);
            proptest::prop_assert_eq!(r.u64().unwrap(), token);
            proptest::prop_assert_eq!(r.samples().unwrap(), samples);
            proptest::prop_assert!(r.finish().is_ok());
        }
    }
}
