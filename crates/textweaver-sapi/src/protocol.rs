//! The framed binary protocol between [`SapiBackend`](crate::SapiBackend)
//! and `textweaver-sapi-host`.
//!
//! It deliberately mirrors the ECI host protocol (`textweaver-eci`), so the
//! two can merge into one engine-host protocol at integration:
//!
//! - Every frame is `len: u32 LE`, then `len` bytes: a one-byte tag followed
//!   by the tag's payload.
//! - Integers are little-endian; strings are a `u32` byte length followed by
//!   UTF-8; sample lists are a `u32` count followed by `i16` samples.
//! - Requests (backend to host, on the host's stdin) have tags `0x01..`;
//!   replies (host to backend, on stdout) have tags `0x81..`.
//!
//! Conversation:
//!
//! 1. The host announces itself with [`Reply::Ready`] (sample rate, its
//!    architecture, the engine name) before anything else.
//! 2. Each [`Request::Speak`] is answered by any number of [`Reply::Audio`]
//!    and [`Reply::Word`] frames followed by exactly one [`Reply::End`]
//!    (preceded by [`Reply::Error`] when synthesis failed). `Word` frames
//!    may arrive before the audio they point into.
//! 3. [`Request::Stop`] aborts the utterance being synthesized and every
//!    `Speak` received before it; each still gets its `End` (status
//!    [`EndStatus::Aborted`]).
//! 4. The host exits cleanly on [`Request::Quit`] or end of input.
//!
//! Voice listing is a separate one-shot run: `textweaver-sapi-host
//! --list-voices [--category onecore]` writes `Ready`, one [`Reply::Voice`]
//! per voice token, and exits. Listing reads the registry only; it never
//! loads an engine.
//!
//! Word positions are UTF-16 code-unit offsets into the utterance text as
//! sent (the host converts UTF-8 to UTF-16 losslessly, so the backend can
//! map them back exactly; see [`crate::words`]). Audio positions are samples
//! from the utterance's first sample.

use std::io::{self, Read, Write};

/// Version of this protocol, reported in [`Reply::Ready`].
pub const PROTOCOL_VERSION: u16 = 1;

/// Largest frame either side accepts (16 MiB). Larger lengths mean a
/// corrupt stream.
pub const MAX_FRAME: usize = 16 * 1024 * 1024;

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
    /// Synthesize `text` as utterance `token`.
    Speak {
        /// Backend-chosen identifier echoed in every reply about this
        /// utterance (never 0).
        token: u64,
        /// The text, UTF-8. The host speaks it as plain text (never as XML).
        text: String,
        /// SAPI pitch, `-10..=10` (`<pitch absmiddle>`); 0 is the voice's
        /// own pitch.
        pitch: i8,
    },
    /// Abort the synthesis in progress and skip every `Speak` received
    /// before this request.
    Stop,
    /// Select a voice by its SAPI token id (a registry path); the empty
    /// string selects the system default voice.
    SetVoice {
        /// The token id.
        token_id: String,
    },
    /// Set the SAPI rate, `-10..=10`, for the next utterances.
    SetRate {
        /// The rate.
        rate: i8,
    },
    /// Exit cleanly.
    Quit,
}

/// One voice token, as the host reads it from the registry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VoiceToken {
    /// The token id (a registry path), passed back in
    /// [`Request::SetVoice`].
    pub token_id: String,
    /// The voice's name (`Attributes\Name`, else the token's description).
    pub name: String,
    /// `Attributes\Language`: hexadecimal LCIDs separated by `;` ("409;9").
    pub language: String,
    /// `Attributes\Gender` ("Male", "Female"), possibly empty.
    pub gender: String,
    /// `Attributes\Vendor`, possibly empty.
    pub vendor: String,
}

/// Host to backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The host is ready.
    Ready {
        /// [`PROTOCOL_VERSION`] of the host.
        protocol: u16,
        /// Audio sample rate in Hz (16-bit mono PCM).
        sample_rate: u32,
        /// The host's architecture: `"x64"` or `"x86"`.
        arch: String,
        /// The engine: `"sapi"`, or `"fake"` for the test engine.
        engine: String,
    },
    /// PCM samples for utterance `token`.
    Audio {
        /// The utterance.
        token: u64,
        /// 16-bit mono samples at the `Ready` sample rate.
        samples: Vec<i16>,
    },
    /// A word boundary (`SPEI_WORD_BOUNDARY`).
    Word {
        /// The utterance.
        token: u64,
        /// First UTF-16 code unit of the word in the utterance text.
        start: u32,
        /// Length of the word in UTF-16 code units (may be 0).
        len: u32,
        /// Samples of this utterance's audio before the word.
        sample: u64,
    },
    /// The utterance is complete (`SPEI_END_INPUT_STREAM`, or aborted).
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
    /// One installed voice (voice listing only).
    Voice(VoiceToken),
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
    pub const SET_RATE: u8 = 0x04;
    pub const QUIT: u8 = 0x05;
    pub const READY: u8 = 0x81;
    pub const AUDIO: u8 = 0x82;
    pub const WORD: u8 = 0x83;
    pub const END: u8 = 0x84;
    pub const ERROR: u8 = 0x85;
    pub const VOICE: u8 = 0x86;
}

/// Frame writer: length placeholder, tag, payload.
struct Enc(Vec<u8>);

impl Enc {
    fn new(tag: u8) -> Self {
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
        Ok(self.take(1)?[0])
    }
    fn i8(&mut self) -> Result<i8, ProtocolError> {
        Ok(i8::from_le_bytes(self.array()?))
    }
    fn u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, ProtocolError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, ProtocolError> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn count(&mut self, item_size: usize) -> Result<usize, ProtocolError> {
        let n = self.u32()? as usize;
        if n.saturating_mul(item_size) > self.0.len() {
            return Err(ProtocolError::Truncated);
        }
        Ok(n)
    }
    fn str(&mut self) -> Result<String, ProtocolError> {
        let n = self.count(1)?;
        let bytes = self.take(n)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ProtocolError::BadUtf8)
    }
    fn samples(&mut self) -> Result<Vec<i16>, ProtocolError> {
        let n = self.count(2)?;
        let bytes = self.take(n * 2)?;
        Ok(bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect())
    }
    fn end(self) -> Result<(), ProtocolError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ProtocolError::Trailing(self.0.len()))
        }
    }
}

impl Request {
    /// Encodes the request as one complete frame (length prefix included).
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Request::Speak { token, text, pitch } => {
                let mut e = Enc::new(tag::SPEAK);
                e.u64(*token);
                e.str(text);
                e.0.extend_from_slice(&pitch.to_le_bytes());
                e.finish()
            }
            Request::Stop => Enc::new(tag::STOP).finish(),
            Request::SetVoice { token_id } => {
                let mut e = Enc::new(tag::SET_VOICE);
                e.str(token_id);
                e.finish()
            }
            Request::SetRate { rate } => {
                let mut e = Enc::new(tag::SET_RATE);
                e.0.extend_from_slice(&rate.to_le_bytes());
                e.finish()
            }
            Request::Quit => Enc::new(tag::QUIT).finish(),
        }
    }

    /// Decodes one frame body (tag and payload, without the length).
    pub fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        let (&t, rest) = body.split_first().ok_or(ProtocolError::BadLength(0))?;
        let mut d = Dec(rest);
        let r = match t {
            tag::SPEAK => Request::Speak {
                token: d.u64()?,
                text: d.str()?,
                pitch: d.i8()?,
            },
            tag::STOP => Request::Stop,
            tag::SET_VOICE => Request::SetVoice { token_id: d.str()? },
            tag::SET_RATE => Request::SetRate { rate: d.i8()? },
            tag::QUIT => Request::Quit,
            other => return Err(ProtocolError::BadTag(other)),
        };
        d.end()?;
        Ok(r)
    }
}

impl Reply {
    /// Encodes the reply as one complete frame (length prefix included).
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Reply::Ready {
                protocol,
                sample_rate,
                arch,
                engine,
            } => {
                let mut e = Enc::new(tag::READY);
                e.u16(*protocol);
                e.u32(*sample_rate);
                e.str(arch);
                e.str(engine);
                e.finish()
            }
            Reply::Audio { token, samples } => {
                let mut e = Enc::new(tag::AUDIO);
                e.0.reserve(12 + samples.len() * 2);
                e.u64(*token);
                e.len(samples.len());
                for s in samples {
                    e.0.extend_from_slice(&s.to_le_bytes());
                }
                e.finish()
            }
            Reply::Word {
                token,
                start,
                len,
                sample,
            } => {
                let mut e = Enc::new(tag::WORD);
                e.u64(*token);
                e.u32(*start);
                e.u32(*len);
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
            Reply::Voice(v) => {
                let mut e = Enc::new(tag::VOICE);
                e.str(&v.token_id);
                e.str(&v.name);
                e.str(&v.language);
                e.str(&v.gender);
                e.str(&v.vendor);
                e.finish()
            }
        }
    }

    /// Decodes one frame body (tag and payload, without the length).
    pub fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        let (&t, rest) = body.split_first().ok_or(ProtocolError::BadLength(0))?;
        let mut d = Dec(rest);
        let r = match t {
            tag::READY => Reply::Ready {
                protocol: d.u16()?,
                sample_rate: d.u32()?,
                arch: d.str()?,
                engine: d.str()?,
            },
            tag::AUDIO => Reply::Audio {
                token: d.u64()?,
                samples: d.samples()?,
            },
            tag::WORD => Reply::Word {
                token: d.u64()?,
                start: d.u32()?,
                len: d.u32()?,
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
            tag::VOICE => Reply::Voice(VoiceToken {
                token_id: d.str()?,
                name: d.str()?,
                language: d.str()?,
                gender: d.str()?,
                vendor: d.str()?,
            }),
            other => return Err(ProtocolError::BadTag(other)),
        };
        d.end()?;
        Ok(r)
    }
}

/// Writes one encoded frame and flushes.
pub fn write_frame(w: &mut impl Write, frame: &[u8]) -> io::Result<()> {
    w.write_all(frame)?;
    w.flush()
}

/// Reads one frame body (tag and payload). `Ok(None)` at a clean end of
/// stream (no bytes of a new frame read).
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
    let n = u32::from_le_bytes(len) as usize;
    if n == 0 || n > MAX_FRAME {
        return Err(ProtocolError::BadLength(n));
    }
    let mut body = vec![0u8; n];
    r.read_exact(&mut body).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            ProtocolError::Truncated
        } else {
            ProtocolError::Io(e)
        }
    })?;
    Ok(Some(body))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requests() -> Vec<Request> {
        vec![
            Request::Speak {
                token: 7,
                text: "Café crème, naïve 😀".into(),
                pitch: -3,
            },
            Request::Stop,
            Request::SetVoice {
                token_id: r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices\Tokens\X".into(),
            },
            Request::SetRate { rate: -10 },
            Request::Quit,
        ]
    }

    fn replies() -> Vec<Reply> {
        vec![
            Reply::Ready {
                protocol: PROTOCOL_VERSION,
                sample_rate: 22050,
                arch: "x86".into(),
                engine: "sapi".into(),
            },
            Reply::Audio {
                token: 1,
                samples: vec![0, -1, i16::MAX, i16::MIN],
            },
            Reply::Word {
                token: 1,
                start: 3,
                len: 5,
                sample: 12_345,
            },
            Reply::End {
                token: 1,
                status: EndStatus::Aborted,
                samples: 99,
            },
            Reply::Error {
                token: 0,
                message: "no voice".into(),
            },
            Reply::Voice(VoiceToken {
                token_id: "id".into(),
                name: "Microsoft David Desktop".into(),
                language: "409".into(),
                gender: "Male".into(),
                vendor: "Microsoft".into(),
            }),
        ]
    }

    #[test]
    fn requests_round_trip() {
        for r in requests() {
            let f = r.encode();
            let body = read_body(&mut &f[..]).unwrap().unwrap();
            assert_eq!(Request::decode(&body).unwrap(), r);
        }
    }

    #[test]
    fn replies_round_trip() {
        for r in replies() {
            let f = r.encode();
            let body = read_body(&mut &f[..]).unwrap().unwrap();
            assert_eq!(Reply::decode(&body).unwrap(), r);
        }
    }

    #[test]
    fn a_stream_of_frames_reads_back_in_order_and_ends_cleanly() {
        let mut stream = Vec::new();
        for r in replies() {
            write_frame(&mut stream, &r.encode()).unwrap();
        }
        let mut cursor = &stream[..];
        let mut got = Vec::new();
        while let Some(body) = read_body(&mut cursor).unwrap() {
            got.push(Reply::decode(&body).unwrap());
        }
        assert_eq!(got, replies());
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
    fn partial_reads_assemble_whole_frames() {
        let mut stream = Vec::new();
        for r in replies() {
            stream.extend(r.encode());
        }
        let mut t = Trickle(&stream);
        let mut n = 0;
        while let Some(body) = read_body(&mut t).unwrap() {
            Reply::decode(&body).unwrap();
            n += 1;
        }
        assert_eq!(n, replies().len());
    }

    #[test]
    fn truncated_and_corrupt_frames_are_errors() {
        let f = Reply::Word {
            token: 1,
            start: 0,
            len: 1,
            sample: 0,
        }
        .encode();
        // Cut inside the length and inside the body.
        assert!(matches!(
            read_body(&mut &f[..2]),
            Err(ProtocolError::Truncated)
        ));
        assert!(matches!(
            read_body(&mut &f[..f.len() - 1]),
            Err(ProtocolError::Truncated)
        ));
        // Zero and oversized lengths.
        assert!(matches!(
            read_body(&mut &[0u8, 0, 0, 0][..]),
            Err(ProtocolError::BadLength(0))
        ));
        assert!(matches!(
            read_body(&mut &[0xff, 0xff, 0xff, 0x7f][..]),
            Err(ProtocolError::BadLength(_))
        ));
        // Unknown tag, trailing bytes, bad status, bad UTF-8.
        assert!(matches!(
            Reply::decode(&[0x7f]),
            Err(ProtocolError::BadTag(0x7f))
        ));
        assert!(matches!(
            Request::decode(&[tag::STOP, 0]),
            Err(ProtocolError::Trailing(1))
        ));
        let mut end = Reply::End {
            token: 1,
            status: EndStatus::Done,
            samples: 0,
        }
        .encode();
        end[4 + 1 + 8] = 9;
        assert!(matches!(
            Reply::decode(&end[4..]),
            Err(ProtocolError::BadValue("end status", 9))
        ));
        let bad = [tag::SET_VOICE, 2, 0, 0, 0, 0xff, 0xfe];
        assert!(matches!(
            Request::decode(&bad),
            Err(ProtocolError::BadUtf8)
        ));
        // A count larger than the frame is truncation, not a huge allocation.
        let huge = [tag::AUDIO, 1, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0x0f];
        assert!(matches!(
            Reply::decode(&huge),
            Err(ProtocolError::Truncated)
        ));
    }
}
