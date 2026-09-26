//! The framed binary protocol between [`EciBackend`](crate::EciBackend) and
//! `textweaver-eci-host`.
//!
//! Every frame is `len: u32 LE`, then `len` bytes: a one-byte tag followed by
//! the tag's payload. Integers are little-endian; strings are a `u32` byte
//! length followed by UTF-8; lists are a `u32` count followed by the items.
//!
//! The backend writes [`Request`]s to the host's stdin; the host writes
//! [`Reply`]s to its stdout. The host announces itself with
//! [`Reply::Ready`] before anything else. Each [`Request::Speak`] is answered
//! by any number of [`Reply::Audio`] and [`Reply::Mark`] frames followed by
//! exactly one [`Reply::End`] (preceded by [`Reply::Error`] when synthesis
//! failed). The host exits cleanly on [`Request::Quit`] or end of input.

use std::io::{self, Read, Write};

/// Version of this protocol, reported in [`Reply::Ready`].
pub const PROTOCOL_VERSION: u16 = 1;

/// Largest frame either side accepts (16 MiB). Larger lengths mean a corrupt
/// stream.
pub const MAX_FRAME: usize = 16 * 1024 * 1024;

/// One piece of an utterance: text to speak or an index mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    /// Text, in UTF-8; the host encodes it for the engine's language.
    Text(String),
    /// An index mark; the host reports it as [`Reply::Mark`] with the sample
    /// offset at which the audio reaches it.
    Index(u32),
}

/// A voice preset as the engine reports it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresetInfo {
    /// The engine's name for the preset.
    pub name: String,
    /// The preset's eight voice parameters, indexed by [`VoiceParam`](crate::VoiceParam).
    pub params: [i32; 8],
}

/// Outcome of one [`Request::Speak`], reported in [`Reply::End`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndStatus {
    /// All audio was delivered.
    Done,
    /// A [`Request::Stop`] cut synthesis short (or skipped it).
    Aborted,
    /// The engine failed; a [`Reply::Error`] preceded this frame.
    Failed,
}

impl EndStatus {
    fn to_byte(self) -> u8 {
        match self {
            EndStatus::Done => 0,
            EndStatus::Aborted => 1,
            EndStatus::Failed => 2,
        }
    }

    fn from_byte(b: u8) -> Result<Self, ProtocolError> {
        match b {
            0 => Ok(EndStatus::Done),
            1 => Ok(EndStatus::Aborted),
            2 => Ok(EndStatus::Failed),
            other => Err(ProtocolError::BadValue("end status", u32::from(other))),
        }
    }
}

/// Backend to host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Synthesize `pieces` as utterance `token`.
    Speak {
        /// Backend-chosen identifier echoed in every reply about this utterance.
        token: u64,
        /// Text and index marks, in order.
        pieces: Vec<Piece>,
    },
    /// Abort the synthesis in progress and skip every `Speak` received
    /// before this request.
    Stop,
    /// Select a language dialect (ECI code) and copy voice preset `preset`
    /// (1..=8; 0 keeps the current voice) into the active voice.
    SetVoice {
        /// `ECILanguageDialect` code.
        dialect: u32,
        /// Preset number, 1..=8, or 0.
        preset: u8,
    },
    /// Set one parameter of the active voice (`ECIVoiceParam`).
    SetVoiceParam {
        /// Parameter number (see [`VoiceParam`](crate::VoiceParam)).
        param: u8,
        /// Value in the engine's units.
        value: i32,
    },
    /// Exit cleanly.
    Quit,
}

/// Host to backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The engine is loaded and ready.
    Ready {
        /// [`PROTOCOL_VERSION`] of the host.
        protocol: u16,
        /// Audio sample rate in Hz (16-bit mono PCM).
        sample_rate: u32,
        /// The engine's version string.
        version: String,
        /// `ECILanguageDialect` codes the engine reports as available (may be
        /// empty when the engine cannot enumerate them).
        dialects: Vec<u32>,
        /// The dialect active at start-up.
        default_dialect: u32,
        /// Voice presets 1..=8 (index 0 is preset 1).
        presets: Vec<PresetInfo>,
    },
    /// PCM samples for utterance `token`.
    Audio {
        /// The utterance.
        token: u64,
        /// 16-bit mono samples at the `Ready` sample rate.
        samples: Vec<i16>,
    },
    /// An index mark was reached.
    Mark {
        /// The utterance.
        token: u64,
        /// The index from [`Piece::Index`].
        index: u32,
        /// Samples of this utterance's audio before the mark.
        sample: u64,
    },
    /// The utterance is complete.
    End {
        /// The utterance.
        token: u64,
        /// How it ended.
        status: EndStatus,
        /// Total samples delivered for the utterance.
        samples: u64,
    },
    /// Something failed. `token` is 0 when no utterance is involved.
    Error {
        /// The utterance, or 0.
        token: u64,
        /// A human-readable description.
        message: String,
    },
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

mod tag {
    pub const SPEAK: u8 = 0x01;
    pub const STOP: u8 = 0x02;
    pub const SET_VOICE: u8 = 0x03;
    pub const SET_VOICE_PARAM: u8 = 0x04;
    pub const QUIT: u8 = 0x05;
    pub const READY: u8 = 0x81;
    pub const AUDIO: u8 = 0x82;
    pub const MARK: u8 = 0x83;
    pub const END: u8 = 0x84;
    pub const ERROR: u8 = 0x85;
}

/// Payload writer.
struct Enc(Vec<u8>);

impl Enc {
    fn new(tag: u8) -> Self {
        // Length placeholder, patched in `finish`.
        Enc(vec![0, 0, 0, 0, tag])
    }
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn len(&mut self, n: usize) {
        self.u32(u32::try_from(n).unwrap_or(u32::MAX));
    }
    fn str(&mut self, s: &str) {
        self.len(s.len());
        self.0.extend_from_slice(s.as_bytes());
    }
    fn finish(mut self) -> Vec<u8> {
        let n = u32::try_from(self.0.len() - 4).unwrap_or(u32::MAX);
        self.0[..4].copy_from_slice(&n.to_le_bytes());
        self.0
    }
}

/// Payload reader over one frame body (after the tag).
struct Dec<'a>(&'a [u8]);

impl<'a> Dec<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ProtocolError> {
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
    fn u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.array::<1>()?[0])
    }
    fn u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, ProtocolError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn i32(&mut self) -> Result<i32, ProtocolError> {
        Ok(i32::from_le_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, ProtocolError> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    /// A list count, checked against the bytes left so a corrupt count
    /// cannot trigger a huge allocation.
    fn count(&mut self, min_item: usize) -> Result<usize, ProtocolError> {
        let n = self.u32()? as usize;
        if n.saturating_mul(min_item.max(1)) > self.0.len() {
            return Err(ProtocolError::Truncated);
        }
        Ok(n)
    }
    fn str(&mut self) -> Result<String, ProtocolError> {
        let n = self.count(1)?;
        let bytes = self.take(n)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ProtocolError::BadUtf8)
    }
    fn done(&self) -> Result<(), ProtocolError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ProtocolError::Trailing(self.0.len()))
        }
    }
}

impl Request {
    /// The frame bytes for this request.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Request::Speak { token, pieces } => {
                let mut e = Enc::new(tag::SPEAK);
                e.u64(*token);
                e.len(pieces.len());
                for p in pieces {
                    match p {
                        Piece::Text(s) => {
                            e.u8(0);
                            e.str(s);
                        }
                        Piece::Index(i) => {
                            e.u8(1);
                            e.u32(*i);
                        }
                    }
                }
                e.finish()
            }
            Request::Stop => Enc::new(tag::STOP).finish(),
            Request::SetVoice { dialect, preset } => {
                let mut e = Enc::new(tag::SET_VOICE);
                e.u32(*dialect);
                e.u8(*preset);
                e.finish()
            }
            Request::SetVoiceParam { param, value } => {
                let mut e = Enc::new(tag::SET_VOICE_PARAM);
                e.u8(*param);
                e.i32(*value);
                e.finish()
            }
            Request::Quit => Enc::new(tag::QUIT).finish(),
        }
    }

    /// Parses one frame body (tag and payload, without the length prefix).
    pub fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        let (&t, rest) = body.split_first().ok_or(ProtocolError::BadLength(0))?;
        let mut d = Dec(rest);
        let r = match t {
            tag::SPEAK => {
                let token = d.u64()?;
                let n = d.count(5)?;
                let mut pieces = Vec::with_capacity(n);
                for _ in 0..n {
                    pieces.push(match d.u8()? {
                        0 => Piece::Text(d.str()?),
                        1 => Piece::Index(d.u32()?),
                        k => return Err(ProtocolError::BadValue("piece kind", u32::from(k))),
                    });
                }
                Request::Speak { token, pieces }
            }
            tag::STOP => Request::Stop,
            tag::SET_VOICE => Request::SetVoice {
                dialect: d.u32()?,
                preset: d.u8()?,
            },
            tag::SET_VOICE_PARAM => Request::SetVoiceParam {
                param: d.u8()?,
                value: d.i32()?,
            },
            tag::QUIT => Request::Quit,
            other => return Err(ProtocolError::BadTag(other)),
        };
        d.done()?;
        Ok(r)
    }
}

impl Reply {
    /// The frame bytes for this reply.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Reply::Ready {
                protocol,
                sample_rate,
                version,
                dialects,
                default_dialect,
                presets,
            } => {
                let mut e = Enc::new(tag::READY);
                e.u16(*protocol);
                e.u32(*sample_rate);
                e.str(version);
                e.len(dialects.len());
                for d in dialects {
                    e.u32(*d);
                }
                e.u32(*default_dialect);
                e.len(presets.len());
                for p in presets {
                    e.str(&p.name);
                    for v in p.params {
                        e.i32(v);
                    }
                }
                e.finish()
            }
            Reply::Audio { token, samples } => encode_audio(*token, samples),
            Reply::Mark {
                token,
                index,
                sample,
            } => {
                let mut e = Enc::new(tag::MARK);
                e.u64(*token);
                e.u32(*index);
                e.u64(*sample);
                e.finish()
            }
            Reply::End {
                token,
                status,
                samples,
            } => {
                let mut e = Enc::new(tag::END);
                e.u64(*token);
                e.u8(status.to_byte());
                e.u64(*samples);
                e.finish()
            }
            Reply::Error { token, message } => {
                let mut e = Enc::new(tag::ERROR);
                e.u64(*token);
                e.str(message);
                e.finish()
            }
        }
    }

    /// Parses one frame body (tag and payload, without the length prefix).
    pub fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        let (&t, rest) = body.split_first().ok_or(ProtocolError::BadLength(0))?;
        let mut d = Dec(rest);
        let r = match t {
            tag::READY => {
                let protocol = d.u16()?;
                let sample_rate = d.u32()?;
                let version = d.str()?;
                let n = d.count(4)?;
                let dialects = (0..n).map(|_| d.u32()).collect::<Result<_, _>>()?;
                let default_dialect = d.u32()?;
                let n = d.count(36)?;
                let mut presets = Vec::with_capacity(n);
                for _ in 0..n {
                    let name = d.str()?;
                    let mut params = [0; 8];
                    for p in &mut params {
                        *p = d.i32()?;
                    }
                    presets.push(PresetInfo { name, params });
                }
                Reply::Ready {
                    protocol,
                    sample_rate,
                    version,
                    dialects,
                    default_dialect,
                    presets,
                }
            }
            tag::AUDIO => {
                let token = d.u64()?;
                let n = d.count(2)?;
                let bytes = d.take(n * 2)?;
                let samples = bytes
                    .chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]))
                    .collect();
                Reply::Audio { token, samples }
            }
            tag::MARK => Reply::Mark {
                token: d.u64()?,
                index: d.u32()?,
                sample: d.u64()?,
            },
            tag::END => Reply::End {
                token: d.u64()?,
                status: EndStatus::from_byte(d.u8()?)?,
                samples: d.u64()?,
            },
            tag::ERROR => Reply::Error {
                token: d.u64()?,
                message: d.str()?,
            },
            other => return Err(ProtocolError::BadTag(other)),
        };
        d.done()?;
        Ok(r)
    }
}

/// Encodes an audio frame straight from a sample slice (the host's hot path;
/// avoids copying into a [`Reply`]).
pub fn encode_audio(token: u64, samples: &[i16]) -> Vec<u8> {
    let mut e = Enc::new(tag::AUDIO);
    e.0.reserve(12 + samples.len() * 2);
    e.u64(token);
    e.len(samples.len());
    for s in samples {
        e.0.extend_from_slice(&s.to_le_bytes());
    }
    e.finish()
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

/// Reads one frame body from a blocking reader. `Ok(None)` on a clean end of
/// stream (no partial frame).
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

    fn requests() -> Vec<Request> {
        vec![
            Request::Speak {
                token: 42,
                pieces: vec![
                    Piece::Index(0),
                    Piece::Text("Café ".into()),
                    Piece::Index(1),
                    Piece::Text("naïve — 日本".into()),
                ],
            },
            Request::Speak {
                token: u64::MAX,
                pieces: vec![],
            },
            Request::Stop,
            Request::SetVoice {
                dialect: 0x0001_0001,
                preset: 2,
            },
            Request::SetVoiceParam {
                param: 6,
                value: -7,
            },
            Request::Quit,
        ]
    }

    fn replies() -> Vec<Reply> {
        vec![
            Reply::Ready {
                protocol: PROTOCOL_VERSION,
                sample_rate: 11025,
                version: "6.1.0.0".into(),
                dialects: vec![0x0001_0000, 0x0004_0000],
                default_dialect: 0x0001_0000,
                presets: vec![PresetInfo {
                    name: "Reed".into(),
                    params: [0, 50, 65, 30, 0, 0, 50, 92],
                }],
            },
            Reply::Audio {
                token: 7,
                samples: vec![0, 1, -1, i16::MAX, i16::MIN],
            },
            Reply::Audio {
                token: 7,
                samples: vec![],
            },
            Reply::Mark {
                token: 7,
                index: 3,
                sample: 12_345,
            },
            Reply::End {
                token: 7,
                status: EndStatus::Aborted,
                samples: 99,
            },
            Reply::Error {
                token: 0,
                message: "no engine".into(),
            },
        ]
    }

    #[test]
    fn requests_round_trip() {
        for r in requests() {
            let bytes = r.encode();
            let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
            assert_eq!(len, bytes.len() - 4);
            assert_eq!(Request::decode(&bytes[4..]).unwrap(), r);
        }
    }

    #[test]
    fn replies_round_trip() {
        for r in replies() {
            let bytes = r.encode();
            assert_eq!(Reply::decode(&bytes[4..]).unwrap(), r);
        }
    }

    #[test]
    fn decoder_reassembles_byte_by_byte() {
        let stream: Vec<u8> = replies().iter().flat_map(Reply::encode).collect();
        let mut dec = FrameDecoder::new();
        let mut got = Vec::new();
        for b in &stream {
            dec.push(std::slice::from_ref(b));
            while let Some(body) = dec.next_body().unwrap() {
                got.push(Reply::decode(&body).unwrap());
            }
        }
        assert_eq!(got, replies());
        assert!(!dec.has_partial());
    }

    #[test]
    fn decoder_handles_odd_chunks() {
        let stream: Vec<u8> = replies().iter().flat_map(Reply::encode).collect();
        for chunk in [2, 3, 7, 13, 1000] {
            let mut dec = FrameDecoder::new();
            let mut got = Vec::new();
            for c in stream.chunks(chunk) {
                dec.push(c);
                while let Some(body) = dec.next_body().unwrap() {
                    got.push(Reply::decode(&body).unwrap());
                }
            }
            assert_eq!(got, replies(), "chunk size {chunk}");
        }
    }

    #[test]
    fn blocking_reader_reads_frames_then_clean_eof() {
        let stream: Vec<u8> = requests().iter().flat_map(Request::encode).collect();
        let mut r = std::io::Cursor::new(stream);
        let mut got = Vec::new();
        while let Some(body) = read_body(&mut r).unwrap() {
            got.push(Request::decode(&body).unwrap());
        }
        assert_eq!(got, requests());
    }

    #[test]
    fn truncated_stream_is_an_error() {
        let bytes = Request::Stop.encode();
        let mut both = bytes.clone();
        both.extend_from_slice(&Request::Quit.encode()[..3]);
        let mut r = std::io::Cursor::new(both);
        assert!(read_body(&mut r).unwrap().is_some());
        assert!(matches!(read_body(&mut r), Err(ProtocolError::Truncated)));
    }

    #[test]
    fn corrupt_frames_are_rejected() {
        let mut dec = FrameDecoder::new();
        dec.push(&u32::MAX.to_le_bytes());
        assert!(matches!(dec.next_body(), Err(ProtocolError::BadLength(_))));
        assert!(matches!(
            Request::decode(&[0x7f]),
            Err(ProtocolError::BadTag(0x7f))
        ));
        // A count larger than the remaining bytes must not allocate.
        let mut body = vec![tag::SPEAK];
        body.extend_from_slice(&1u64.to_le_bytes());
        body.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            Request::decode(&body),
            Err(ProtocolError::Truncated)
        ));
        let mut stop = Request::Stop.encode()[4..].to_vec();
        stop.push(0);
        assert!(matches!(
            Request::decode(&stop),
            Err(ProtocolError::Trailing(1))
        ));
    }

    proptest::proptest! {
        #[test]
        fn speak_round_trips(token: u64, words in proptest::collection::vec(".{0,12}", 0..8)) {
            let pieces: Vec<Piece> = words
                .into_iter()
                .enumerate()
                .flat_map(|(i, w)| [Piece::Index(i as u32), Piece::Text(w)])
                .collect();
            let r = Request::Speak { token, pieces };
            let bytes = r.encode();
            let mut dec = FrameDecoder::new();
            dec.push(&bytes);
            let body = dec.next_body().unwrap().unwrap();
            proptest::prop_assert_eq!(Request::decode(&body).unwrap(), r);
        }
    }
}
