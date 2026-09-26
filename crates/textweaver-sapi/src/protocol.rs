//! The SAPI messages of the engine-host protocol between `SapiBackend` and
//! `textweaver-sapi-host` (ADR-0012).
//!
//! Framing, the shared messages (Speak, Stop, Quit, the `Ready` header,
//! Audio, End, Error), and the version rule live in
//! [`textweaver_enginehost::protocol`]; this module defines the SAPI
//! payloads and messages:
//!
//! - `Speak` (0x01) carries the text and a SAPI pitch;
//! - `SetVoice` (0x03) selects a voice token; `SetRate` (0x04) sets the
//!   SAPI rate;
//! - `Ready` (0x81) adds the host's architecture and engine name to the
//!   shared header;
//! - `Word` (0x83) reports a word boundary as a UTF-16 range and its sample
//!   offset;
//! - `Voice` (0x86) describes one voice token (voice listing only).
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

pub use textweaver_enginehost::protocol::{
    EndStatus, MAX_FRAME, PROTOCOL_VERSION, ProtocolError, read_body, write_frame,
};
use textweaver_enginehost::protocol::{
    FrameReader, FrameWriter, Message, ReadyHeader, encode_audio, encode_end, encode_error,
    encode_quit, encode_stop,
};

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

/// The shared tags plus the SAPI ones.
mod tag {
    pub use textweaver_enginehost::protocol::tag::*;
    pub const SET_VOICE: u8 = 0x03;
    pub const SET_RATE: u8 = 0x04;
    pub const VOICE: u8 = 0x86;
}

impl Request {
    /// Encodes the request as one complete frame (length prefix included).
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Request::Speak { token, text, pitch } => {
                let mut e = FrameWriter::new(tag::SPEAK);
                e.u64(*token);
                e.str(text);
                e.i8(*pitch);
                e.finish()
            }
            Request::Stop => encode_stop(),
            Request::SetVoice { token_id } => {
                let mut e = FrameWriter::new(tag::SET_VOICE);
                e.str(token_id);
                e.finish()
            }
            Request::SetRate { rate } => {
                let mut e = FrameWriter::new(tag::SET_RATE);
                e.i8(*rate);
                e.finish()
            }
            Request::Quit => encode_quit(),
        }
    }

    /// Decodes one frame body (tag and payload, without the length).
    pub fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        let (t, mut d) = FrameReader::open(body)?;
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
        d.finish()?;
        Ok(r)
    }
}

impl Message for Request {
    fn encode(&self) -> Vec<u8> {
        Request::encode(self)
    }
    fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        Request::decode(body)
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
                let mut e = FrameWriter::new(tag::READY);
                ReadyHeader {
                    protocol: *protocol,
                    sample_rate: *sample_rate,
                }
                .write(&mut e);
                e.str(arch);
                e.str(engine);
                e.finish()
            }
            Reply::Audio { token, samples } => encode_audio(*token, samples),
            Reply::Word {
                token,
                start,
                len,
                sample,
            } => {
                let mut e = FrameWriter::new(tag::WORD);
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
            } => encode_end(*token, *status, *samples),
            Reply::Error { token, message } => encode_error(*token, message),
            Reply::Voice(v) => {
                let mut e = FrameWriter::new(tag::VOICE);
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
        let (t, mut d) = FrameReader::open(body)?;
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
        d.finish()?;
        Ok(r)
    }
}

impl Message for Reply {
    fn encode(&self) -> Vec<u8> {
        Reply::encode(self)
    }
    fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        Reply::decode(body)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

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
        assert!(matches!(Request::decode(&bad), Err(ProtocolError::BadUtf8)));
        // A count larger than the frame is truncation, not a huge allocation.
        let huge = [tag::AUDIO, 1, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0x0f];
        assert!(matches!(
            Reply::decode(&huge),
            Err(ProtocolError::Truncated)
        ));
    }
}
