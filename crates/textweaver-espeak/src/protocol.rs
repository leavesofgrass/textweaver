//! The eSpeak NG messages of the engine-host protocol between
//! [`EspeakHostBackend`](crate::EspeakHostBackend) and
//! `textweaver-espeak-host` (ADR-0012).
//!
//! Framing, the shared messages (Speak, Stop, Quit, the `Ready` header,
//! Audio, End, Error), and the version rule live in
//! [`textweaver_enginehost::protocol`]; this module defines the eSpeak NG
//! payloads:
//!
//! | tag  | message  | eSpeak NG payload |
//! |------|----------|-------------------|
//! | 0x01 | Speak    | `token: u64`, `kind: u8` (0: text, 1: one character, spoken by name), `text: str` |
//! | 0x03 | SetVoice | `voice: str` (empty: the default, `en-us`), `rate: u16` (wpm), `pitch: i8` (semitones) |
//! | 0x81 | Ready    | shared header, then `version: str`, `engine: str` (`espeak-ng` or `fake`), voices: `u32` count, each `id: str`, `name: str`, `gender: u8` (0 unknown, 1 male, 2 female), languages: `u32` count of `str` |
//! | 0x83 | Word     | `token: u64`, `start: u32`, `end: u32` (the word's byte range in the text), `sample: u64` |
//!
//! eSpeak NG reports each word's place in the text itself, so a Speak
//! carries plain text, not the index marks of ECI and DECtalk, and a Word
//! carries the byte range the highlight covers. Volume is not sent: the
//! backend applies it as a playback gain, as for DECtalk. The version
//! stays [`PROTOCOL_VERSION`] 1: eSpeak NG adds a host of its own and
//! changes no existing frame.
//!
//! Conversation: the host announces itself with [`Reply::Ready`] (or sends
//! one [`Reply::Error`] and exits if eSpeak NG cannot start). Each
//! [`Request::Speak`] is answered by any number of [`Reply::Audio`] and
//! [`Reply::Word`] frames and exactly one [`Reply::End`] (preceded by
//! [`Reply::Error`] when synthesis failed). [`Request::SetVoice`] applies
//! to the utterances that follow it. The host exits cleanly on
//! [`Request::Quit`] or end of input.

use std::ops::Range;

pub use textweaver_enginehost::protocol::Message;
pub use textweaver_enginehost::protocol::{
    EndStatus, FrameDecoder, MAX_FRAME, PROTOCOL_VERSION, ProtocolError, encode_audio, read_body,
    write_frame,
};
use textweaver_enginehost::protocol::{
    FrameReader, FrameWriter, ReadyHeader, encode_end, encode_error, encode_quit, encode_stop,
};

/// A voice as the host lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceEntry {
    /// The identifier to select it with (`gmw/en-US`).
    pub id: String,
    /// Its name.
    pub name: String,
    /// 0 unknown, 1 male, 2 female (eSpeak NG's values).
    pub gender: u8,
    /// Language tags, most suitable first.
    pub languages: Vec<String>,
}

/// Backend to host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Synthesize `text` as utterance `token`.
    Speak {
        /// Backend-chosen identifier echoed in every reply about this
        /// utterance (never 0).
        token: u64,
        /// Speak the text's one character by name (eSpeak NG's
        /// `espeak_Char`) instead of reading the text.
        character: bool,
        /// The text, UTF-8.
        text: String,
    },
    /// Abort the synthesis in progress and skip every `Speak` received
    /// before this request.
    Stop,
    /// The voice for the utterances that follow.
    SetVoice {
        /// Voice identifier or name; empty for the default (`en-us`).
        voice: String,
        /// Speaking rate in words per minute (eSpeak NG accepts 80..=450).
        rate: u16,
        /// Pitch offset in semitones.
        pitch: i8,
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
        /// The library the host loaded, as far as it is known.
        version: String,
        /// `espeak-ng` for the real library, `fake` for the test engine.
        engine: String,
        /// The installed voices.
        voices: Vec<VoiceEntry>,
    },
    /// PCM samples for utterance `token`.
    Audio {
        /// The utterance.
        token: u64,
        /// 16-bit mono samples at the `Ready` sample rate.
        samples: Vec<i16>,
    },
    /// A word starts.
    Word {
        /// The utterance.
        token: u64,
        /// The word's byte range in the utterance's text.
        range: Range<u32>,
        /// Samples of this utterance's audio before the word.
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

/// The shared tags plus the eSpeak NG one.
mod tag {
    pub use textweaver_enginehost::protocol::tag::*;
    pub const SET_VOICE: u8 = 0x03;
}

impl Message for Request {
    fn encode(&self) -> Vec<u8> {
        match self {
            Request::Speak {
                token,
                character,
                text,
            } => {
                let mut e = FrameWriter::new(tag::SPEAK);
                e.u64(*token);
                e.u8(u8::from(*character));
                e.str(text);
                e.finish()
            }
            Request::Stop => encode_stop(),
            Request::SetVoice { voice, rate, pitch } => {
                let mut e = FrameWriter::new(tag::SET_VOICE);
                e.str(voice);
                e.u16(*rate);
                e.i8(*pitch);
                e.finish()
            }
            Request::Quit => encode_quit(),
        }
    }

    fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        let (t, mut d) = FrameReader::open(body)?;
        let r = match t {
            tag::SPEAK => Request::Speak {
                token: d.u64()?,
                character: match d.u8()? {
                    0 => false,
                    1 => true,
                    k => return Err(ProtocolError::BadValue("speak kind", u32::from(k))),
                },
                text: d.str()?,
            },
            tag::STOP => Request::Stop,
            tag::SET_VOICE => Request::SetVoice {
                voice: d.str()?,
                rate: d.u16()?,
                pitch: d.i8()?,
            },
            tag::QUIT => Request::Quit,
            other => return Err(ProtocolError::BadTag(other)),
        };
        d.finish()?;
        Ok(r)
    }
}

impl Message for Reply {
    fn encode(&self) -> Vec<u8> {
        match self {
            Reply::Ready {
                protocol,
                sample_rate,
                version,
                engine,
                voices,
            } => {
                let mut e = FrameWriter::new(tag::READY);
                ReadyHeader {
                    protocol: *protocol,
                    sample_rate: *sample_rate,
                }
                .write(&mut e);
                e.str(version);
                e.str(engine);
                e.len(voices.len());
                for v in voices {
                    e.str(&v.id);
                    e.str(&v.name);
                    e.u8(v.gender);
                    e.len(v.languages.len());
                    for l in &v.languages {
                        e.str(l);
                    }
                }
                e.finish()
            }
            Reply::Audio { token, samples } => encode_audio(*token, samples),
            Reply::Word {
                token,
                range,
                sample,
            } => {
                let mut e = FrameWriter::new(tag::WORD);
                e.u64(*token);
                e.u32(range.start);
                e.u32(range.end);
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

    fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        let (t, mut d) = FrameReader::open(body)?;
        let r = match t {
            tag::READY => {
                let protocol = d.u16()?;
                let sample_rate = d.u32()?;
                let version = d.str()?;
                let engine = d.str()?;
                // A voice takes at least two empty strings, a byte, and an
                // empty list: 13 bytes.
                let n = d.count(13)?;
                let mut voices = Vec::with_capacity(n);
                for _ in 0..n {
                    let id = d.str()?;
                    let name = d.str()?;
                    let gender = d.u8()?;
                    let m = d.count(4)?;
                    let mut languages = Vec::with_capacity(m);
                    for _ in 0..m {
                        languages.push(d.str()?);
                    }
                    voices.push(VoiceEntry {
                        id,
                        name,
                        gender,
                        languages,
                    });
                }
                Reply::Ready {
                    protocol,
                    sample_rate,
                    version,
                    engine,
                    voices,
                }
            }
            tag::AUDIO => Reply::Audio {
                token: d.u64()?,
                samples: d.samples()?,
            },
            tag::WORD => {
                let token = d.u64()?;
                let start = d.u32()?;
                let end = d.u32()?;
                if end < start {
                    return Err(ProtocolError::BadValue("word end", end));
                }
                Reply::Word {
                    token,
                    range: start..end,
                    sample: d.u64()?,
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<M: Message + PartialEq + std::fmt::Debug>(m: &M) {
        let f = m.encode();
        let mut dec = FrameDecoder::new();
        dec.push(&f);
        let body = dec.next_body().unwrap().unwrap();
        assert_eq!(&M::decode(&body).unwrap(), m);
    }

    #[test]
    fn every_message_round_trips() {
        for r in [
            Request::Speak {
                token: 7,
                character: false,
                text: "Caf\u{e9} pour \u{c9}mile, \u{65e5}\u{672c}".into(),
            },
            Request::Speak {
                token: 8,
                character: true,
                text: "\u{e9}".into(),
            },
            Request::Stop,
            Request::SetVoice {
                voice: "gmw/en-US".into(),
                rate: 450,
                pitch: -12,
            },
            Request::Quit,
        ] {
            round_trip(&r);
        }
        for r in [
            Reply::Ready {
                protocol: PROTOCOL_VERSION,
                sample_rate: 22050,
                version: "1.52".into(),
                engine: "espeak-ng".into(),
                voices: vec![
                    VoiceEntry {
                        id: "gmw/en-US".into(),
                        name: "English (America)".into(),
                        gender: 1,
                        languages: vec!["en-us".into(), "en".into()],
                    },
                    VoiceEntry {
                        id: String::new(),
                        name: String::new(),
                        gender: 0,
                        languages: vec![],
                    },
                ],
            },
            Reply::Audio {
                token: 3,
                samples: vec![0, -1, i16::MAX],
            },
            Reply::Word {
                token: 3,
                range: 6..10,
                sample: 4410,
            },
            Reply::End {
                token: 3,
                status: EndStatus::Aborted,
                samples: 9,
            },
            Reply::Error {
                token: 0,
                message: "no voice".into(),
            },
        ] {
            round_trip(&r);
        }
    }

    #[test]
    fn the_ready_header_is_shared() {
        let f = Reply::Ready {
            protocol: PROTOCOL_VERSION,
            sample_rate: 22050,
            version: String::new(),
            engine: "fake".into(),
            voices: vec![],
        }
        .encode();
        let h = ReadyHeader::peek(&f[4..]).unwrap();
        assert_eq!((h.protocol, h.sample_rate), (PROTOCOL_VERSION, 22050));
    }

    #[test]
    fn bad_values_are_errors() {
        let mut f = Request::Speak {
            token: 1,
            character: false,
            text: String::new(),
        }
        .encode();
        f[4 + 1 + 8] = 9;
        assert!(matches!(
            Request::decode(&f[4..]),
            Err(ProtocolError::BadValue("speak kind", 9))
        ));
        let f = Reply::Word {
            token: 1,
            range: 5..9,
            sample: 0,
        }
        .encode();
        let mut body = f[4..].to_vec();
        body[13..17].copy_from_slice(&1u32.to_le_bytes());
        assert!(matches!(
            Reply::decode(&body),
            Err(ProtocolError::BadValue("word end", 1))
        ));
        assert!(matches!(
            Reply::decode(&[0xee]),
            Err(ProtocolError::BadTag(0xee))
        ));
    }
}
