//! The DECtalk messages of the engine-host protocol between
//! [`DectalkBackend`](crate::DectalkBackend) and `textweaver-dectalk-host`
//! (ADR-0012, ADR-0021).
//!
//! Framing, the shared messages (Speak, Stop, Quit, the `Ready` header,
//! Audio, End, Error), and the version rule live in
//! [`textweaver_enginehost::protocol`]; this module defines the DECtalk
//! payloads:
//!
//! | tag  | message  | DECtalk payload |
//! |------|----------|-----------------|
//! | 0x01 | Speak    | `token: u64`, pieces: `u32` count, each `kind: u8` (0: text `str`, 1: index mark `u32`) |
//! | 0x03 | SetVoice | `speaker: u8` (0..=8, [`Speaker`](crate::Speaker) order), `rate: u16` (wpm), `pitch: u16` (average pitch in Hz, 0 = the speaker's own) |
//! | 0x81 | Ready    | shared header, then `version: str`, `engine: str` (`dectalk` or `fake`) |
//! | 0x83 | Mark     | `token: u64`, `index: u32`, `sample: u64` |
//!
//! The payloads of Speak and Mark are laid out exactly as ECI's, so the
//! two engines share one idea of an utterance: text pieces and index marks.
//! The version stays [`PROTOCOL_VERSION`] 1: DECtalk adds a host of its
//! own and changes no existing frame.
//!
//! Conversation: the host announces itself with [`Reply::Ready`] (or sends
//! one [`Reply::Error`] and exits if DECtalk cannot start). Each
//! [`Request::Speak`] is answered by any number of [`Reply::Audio`] and
//! [`Reply::Mark`] frames and exactly one [`Reply::End`] (preceded by
//! [`Reply::Error`] when synthesis failed). [`Request::SetVoice`] applies to
//! the utterances that follow it. The host exits cleanly on
//! [`Request::Quit`] or end of input.

pub use textweaver_enginehost::protocol::{
    EndStatus, FrameDecoder, MAX_FRAME, PROTOCOL_VERSION, ProtocolError, encode_audio, read_body,
    write_frame,
};
use textweaver_enginehost::protocol::{
    FrameReader, FrameWriter, Message, ReadyHeader, encode_end, encode_error, encode_quit,
    encode_stop,
};

/// One piece of an utterance: text to speak or an index mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    /// Text, in UTF-8; the host encodes it for DECtalk.
    Text(String),
    /// An index mark; the host reports it as [`Reply::Mark`] with the sample
    /// offset at which the audio reaches it.
    Index(u32),
}

/// Backend to host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Synthesize `pieces` as utterance `token`.
    Speak {
        /// Backend-chosen identifier echoed in every reply about this
        /// utterance (never 0).
        token: u64,
        /// Text and index marks, in order.
        pieces: Vec<Piece>,
    },
    /// Abort the synthesis in progress and skip every `Speak` received
    /// before this request.
    Stop,
    /// The voice for the utterances that follow.
    SetVoice {
        /// The speaker, 0..=8 in [`Speaker`](crate::Speaker) order.
        speaker: u8,
        /// Speaking rate in words per minute (DECtalk accepts 75..=600).
        rate: u16,
        /// Average pitch in Hz; 0 keeps the speaker's own.
        pitch: u16,
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
        /// The engine's version, as far as the library reports it.
        version: String,
        /// `dectalk` for the real library, `fake` for the test engine.
        engine: String,
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

/// The shared tags plus the DECtalk ones.
mod tag {
    pub use textweaver_enginehost::protocol::tag::*;
    pub const SET_VOICE: u8 = 0x03;
    pub const MARK: u8 = WORD;
}

impl Request {
    /// The frame bytes for this request.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Request::Speak { token, pieces } => {
                let mut e = FrameWriter::new(tag::SPEAK);
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
            Request::Stop => encode_stop(),
            Request::SetVoice {
                speaker,
                rate,
                pitch,
            } => {
                let mut e = FrameWriter::new(tag::SET_VOICE);
                e.u8(*speaker);
                e.u16(*rate);
                e.u16(*pitch);
                e.finish()
            }
            Request::Quit => encode_quit(),
        }
    }

    /// Parses one frame body (tag and payload, without the length prefix).
    pub fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        let (t, mut d) = FrameReader::open(body)?;
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
                speaker: d.u8()?,
                rate: d.u16()?,
                pitch: d.u16()?,
            },
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
    /// The frame bytes for this reply.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Reply::Ready {
                protocol,
                sample_rate,
                version,
                engine,
            } => {
                let mut e = FrameWriter::new(tag::READY);
                ReadyHeader {
                    protocol: *protocol,
                    sample_rate: *sample_rate,
                }
                .write(&mut e);
                e.str(version);
                e.str(engine);
                e.finish()
            }
            Reply::Audio { token, samples } => encode_audio(*token, samples),
            Reply::Mark {
                token,
                index,
                sample,
            } => {
                let mut e = FrameWriter::new(tag::MARK);
                e.u64(*token);
                e.u32(*index);
                e.u64(*sample);
                e.finish()
            }
            Reply::End {
                token,
                status,
                samples,
            } => encode_end(*token, *status, *samples),
            Reply::Error { token, message } => encode_error(*token, message),
        }
    }

    /// Parses one frame body (tag and payload, without the length prefix).
    pub fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        let (t, mut d) = FrameReader::open(body)?;
        let r = match t {
            tag::READY => Reply::Ready {
                protocol: d.u16()?,
                sample_rate: d.u32()?,
                version: d.str()?,
                engine: d.str()?,
            },
            tag::AUDIO => Reply::Audio {
                token: d.u64()?,
                samples: d.samples()?,
            },
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
    use textweaver_enginehost::protocol::ReadyHeader;

    use super::*;

    fn requests() -> Vec<Request> {
        vec![
            Request::Speak {
                token: 7,
                pieces: vec![
                    Piece::Index(0),
                    Piece::Text("Café ".into()),
                    Piece::Index(1),
                    Piece::Text("日本".into()),
                ],
            },
            Request::Speak {
                token: 8,
                pieces: vec![],
            },
            Request::Stop,
            Request::SetVoice {
                speaker: 4,
                rate: 180,
                pitch: 0,
            },
            Request::SetVoice {
                speaker: 8,
                rate: 600,
                pitch: 350,
            },
            Request::Quit,
        ]
    }

    fn replies() -> Vec<Reply> {
        vec![
            Reply::Ready {
                protocol: PROTOCOL_VERSION,
                sample_rate: 11025,
                version: "4.61".into(),
                engine: "dectalk".into(),
            },
            Reply::Audio {
                token: 7,
                samples: vec![0, 1, -1, i16::MAX, i16::MIN],
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
            let f = r.encode();
            assert_eq!(Request::decode(&f[4..]).unwrap(), r);
        }
    }

    #[test]
    fn replies_round_trip() {
        for r in replies() {
            let f = r.encode();
            assert_eq!(Reply::decode(&f[4..]).unwrap(), r);
        }
    }

    #[test]
    fn ready_starts_with_the_shared_header() {
        let f = replies()[0].encode();
        assert_eq!(
            ReadyHeader::peek(&f[4..]),
            Some(ReadyHeader {
                protocol: PROTOCOL_VERSION,
                sample_rate: 11025
            })
        );
    }

    #[test]
    fn speak_and_mark_share_ecis_layout() {
        // The same bytes ECI's host would read: token, count, kind, payload.
        let f = Request::Speak {
            token: 1,
            pieces: vec![Piece::Index(2)],
        }
        .encode();
        assert_eq!(
            &f[4..],
            [0x01, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 2, 0, 0, 0]
        );
        let m = Reply::Mark {
            token: 1,
            index: 2,
            sample: 3,
        }
        .encode();
        assert_eq!(m[4], 0x83);
        assert_eq!(m.len(), 4 + 1 + 8 + 4 + 8);
    }

    #[test]
    fn corrupt_frames_are_refused() {
        assert!(matches!(
            Request::decode(&[0x7e]),
            Err(ProtocolError::BadTag(0x7e))
        ));
        assert!(matches!(
            Request::decode(&[0x01, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 9, 0, 0, 0, 0]),
            Err(ProtocolError::BadValue("piece kind", 9))
        ));
        let mut f = Request::SetVoice {
            speaker: 1,
            rate: 2,
            pitch: 3,
        }
        .encode();
        f.push(0);
        assert!(matches!(
            Request::decode(&f[4..]),
            Err(ProtocolError::Trailing(1))
        ));
        assert!(matches!(
            Reply::decode(&[0x84, 1, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0]),
            Err(ProtocolError::BadValue("end status", 7))
        ));
        assert!(Reply::decode(&[0x86]).is_err());
    }
}
