//! The ECI messages of the engine-host protocol between
//! [`EciBackend`](crate::EciBackend) and `textweaver-eci-host` (ADR-0012).
//!
//! Framing, the shared messages (Speak, Stop, Quit, the `Ready` header,
//! Audio, End, Error), and the version rule live in
//! [`textweaver_enginehost::protocol`]; this module defines the ECI
//! payloads and messages:
//!
//! - `Speak` (0x01) carries a list of [`Piece`]s: text and index marks;
//! - `SetVoice` (0x03) selects a dialect and preset; `SetVoiceParam`
//!   (0x04) sets one voice parameter;
//! - `Ready` (0x81) adds the engine version, dialects, and presets to the
//!   shared header;
//! - `Mark` (0x83, the shared word-position tag) reports an index mark and
//!   its sample offset;
//! - `Dictionary` (0x86) reports one pronunciation dictionary file.
//!
//! The backend writes [`Request`]s to the host's stdin; the host writes
//! [`Reply`]s to its stdout. The host announces itself with
//! [`Reply::Ready`], preceded only by [`Reply::Dictionary`] reports for the
//! start-up language (and by one [`Reply::Error`] if the engine cannot
//! start). A language change may report more dictionaries later. Each
//! [`Request::Speak`] is answered by any number of [`Reply::Audio`] and
//! [`Reply::Mark`] frames followed by exactly one [`Reply::End`] (preceded
//! by [`Reply::Error`] when synthesis failed). The host exits cleanly on
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
    /// A pronunciation dictionary file was loaded (or failed to load) for
    /// a language.
    Dictionary {
        /// `ECILanguageDialect` code.
        dialect: u32,
        /// ECI dictionary volume (0 main, 1 root, 2 abbreviation).
        volume: u8,
        /// `ECIDictError` from `eciLoadDict` (0 = loaded), or -1 when the
        /// library lacks the dictionary calls.
        status: i32,
        /// The file.
        path: String,
    },
    /// Something failed. `token` is 0 when no utterance is involved.
    Error {
        /// The utterance, or 0.
        token: u64,
        /// A human-readable description.
        message: String,
    },
}

/// The shared tags plus the ECI ones.
mod tag {
    pub use textweaver_enginehost::protocol::tag::*;
    pub const SET_VOICE: u8 = 0x03;
    pub const SET_VOICE_PARAM: u8 = 0x04;
    pub const MARK: u8 = WORD;
    pub const DICTIONARY: u8 = 0x86;
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
            Request::SetVoice { dialect, preset } => {
                let mut e = FrameWriter::new(tag::SET_VOICE);
                e.u32(*dialect);
                e.u8(*preset);
                e.finish()
            }
            Request::SetVoiceParam { param, value } => {
                let mut e = FrameWriter::new(tag::SET_VOICE_PARAM);
                e.u8(*param);
                e.i32(*value);
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
                dialects,
                default_dialect,
                presets,
            } => {
                let mut e = FrameWriter::new(tag::READY);
                ReadyHeader {
                    protocol: *protocol,
                    sample_rate: *sample_rate,
                }
                .write(&mut e);
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
            Reply::Dictionary {
                dialect,
                volume,
                status,
                path,
            } => {
                let mut e = FrameWriter::new(tag::DICTIONARY);
                e.u32(*dialect);
                e.u8(*volume);
                e.i32(*status);
                e.str(path);
                e.finish()
            }
            Reply::Error { token, message } => encode_error(*token, message),
        }
    }

    /// Parses one frame body (tag and payload, without the length prefix).
    pub fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        let (t, mut d) = FrameReader::open(body)?;
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
            tag::DICTIONARY => Reply::Dictionary {
                dialect: d.u32()?,
                volume: d.u8()?,
                status: d.i32()?,
                path: d.str()?,
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
            Reply::Dictionary {
                dialect: 0x0001_0000,
                volume: 1,
                status: 0,
                path: "/x/ENURoot.dic".into(),
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
