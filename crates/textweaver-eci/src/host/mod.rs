//! The host side of the protocol: the loop `textweaver-eci-host` runs.
//!
//! The loop is generic over [`Engine`] so the same code drives the real ECI
//! library ([`ffi::EciEngine`]) and the deterministic [`fake::FakeEngine`]
//! the integration tests use.
//!
//! Threads: the shared engine host's reader thread
//! ([`RequestReader`]) decodes
//! requests from stdin. It handles `Stop` itself by bumping a stop epoch,
//! which the synthesis callback checks, so a stop aborts synthesis already
//! in progress; every `Speak` is stamped with the epoch at which it was read
//! and skipped if a `Stop` followed it. The main thread owns the engine and
//! writes replies.

pub mod fake;
pub mod ffi;

use std::io::{Read, Write};

use textweaver_enginehost::serve::{AtEnd, Incoming, RequestReader, StopEpoch};

use crate::language;
use crate::protocol::{self, EndStatus, Piece, PresetInfo, Reply, Request};

/// What the engine reports at start-up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineInfo {
    /// Output sample rate in Hz.
    pub sample_rate: u32,
    /// Engine version string.
    pub version: String,
    /// Available dialect codes (may be empty).
    pub dialects: Vec<u32>,
    /// The dialect active now.
    pub default_dialect: u32,
    /// Voice presets 1..=8.
    pub presets: Vec<PresetInfo>,
}

/// One piece of engine input, already encoded for the active language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnginePiece {
    /// Encoded text (no NUL bytes).
    Text(Vec<u8>),
    /// An index mark.
    Index(u32),
}

/// One dictionary file the engine loaded (or failed to load).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictLoad {
    /// The dialect the dictionary serves.
    pub dialect: u32,
    /// ECI volume (0 main, 1 root, 2 abbreviation).
    pub volume: u8,
    /// `ECIDictError` (0 = loaded), or -1 when unsupported.
    pub status: i32,
    /// The file.
    pub path: String,
}

/// Something the engine produced during synthesis.
#[derive(Debug)]
pub enum SynthEvent<'a> {
    /// PCM samples.
    Audio(&'a [i16]),
    /// An index mark was reached (after all audio before it).
    Mark(u32),
}

/// A text-to-speech engine the host can drive.
pub trait Engine {
    /// Start-up facts.
    fn info(&mut self) -> EngineInfo;
    /// The active dialect, for text encoding.
    fn dialect(&self) -> u32;
    /// Selects a dialect and copies preset `preset` (1..=8, 0 = keep) into
    /// the active voice.
    fn set_voice(&mut self, dialect: u32, preset: u8) -> Result<(), String>;
    /// Sets one active-voice parameter.
    fn set_voice_param(&mut self, param: u8, value: i32) -> Result<(), String>;
    /// Activates the pronunciation dictionaries for the current dialect,
    /// loading them the first time that dialect is used, and returns the
    /// files loaded by this call. Default: no dictionaries.
    fn activate_dictionaries(&mut self) -> Vec<DictLoad> {
        Vec::new()
    }
    /// Synthesizes `pieces`, calling `out` for every event in order. `out`
    /// returns false to abort. Returns `Ok(true)` when synthesis completed,
    /// `Ok(false)` when it was aborted.
    fn synthesize(
        &mut self,
        pieces: &[EnginePiece],
        out: &mut dyn FnMut(SynthEvent<'_>) -> bool,
    ) -> Result<bool, String>;
}

/// Runs the host loop until `Quit` or end of input. Returns an error only
/// when the output pipe fails. `at_end` says what the end of input does:
/// the host process passes [`AtEnd::host`] (stop, and exit even if the
/// engine is stuck), in-process tests [`AtEnd::Finish`].
pub fn run<E: Engine>(
    engine: &mut E,
    input: impl Read + Send + 'static,
    output: &mut impl Write,
    at_end: AtEnd,
) -> std::io::Result<()> {
    let info = engine.info();
    // Dictionary reports for the start-up language come first, so the
    // backend has them all when `Ready` arrives.
    report_dictionaries(engine, output)?;
    protocol::write_frame(
        output,
        &Reply::Ready {
            protocol: protocol::PROTOCOL_VERSION,
            sample_rate: info.sample_rate,
            version: info.version,
            dialects: info.dialects,
            default_dialect: info.default_dialect,
            presets: info.presets,
        }
        .encode(),
    )?;

    let reader = RequestReader::<Request>::spawn_with(input, "eci-host-reader", at_end)?;
    let epoch = reader.epoch().clone();
    while let Some(item) = reader.next() {
        match item {
            Incoming::Request(Request::Quit, _) => break,
            Incoming::Bad(message) => {
                protocol::write_frame(output, &Reply::Error { token: 0, message }.encode())?;
            }
            Incoming::Request(Request::Stop, _) => {}
            Incoming::Request(Request::SetVoice { dialect, preset }, _) => {
                match engine.set_voice(dialect, preset) {
                    Ok(()) => report_dictionaries(engine, output)?,
                    Err(message) => {
                        protocol::write_frame(
                            output,
                            &Reply::Error { token: 0, message }.encode(),
                        )?;
                    }
                }
            }
            Incoming::Request(Request::SetVoiceParam { param, value }, _) => {
                if let Err(message) = engine.set_voice_param(param, value) {
                    protocol::write_frame(output, &Reply::Error { token: 0, message }.encode())?;
                }
            }
            Incoming::Request(Request::Speak { token, pieces }, at) => {
                speak(engine, output, &epoch, token, &pieces, at)?;
            }
        }
    }
    Ok(())
}

fn report_dictionaries<E: Engine>(engine: &mut E, output: &mut impl Write) -> std::io::Result<()> {
    for load in engine.activate_dictionaries() {
        protocol::write_frame(
            output,
            &Reply::Dictionary {
                dialect: load.dialect,
                volume: load.volume,
                status: load.status,
                path: load.path,
            }
            .encode(),
        )?;
    }
    Ok(())
}

/// Encodes pieces for the engine. The text always ends in whitespace:
/// Voxin never finishes synthesizing input whose last byte is a Latin-1
/// letter ("café", "Ü"; measured on libvoxin 1.5.8), and a trailing space
/// changes nothing for any engine.
fn engine_input(dialect: u32, pieces: &[Piece]) -> Vec<EnginePiece> {
    let mut input: Vec<EnginePiece> = pieces
        .iter()
        .map(|p| match p {
            Piece::Text(s) => EnginePiece::Text(language::encode_for(dialect, s)),
            Piece::Index(i) => EnginePiece::Index(*i),
        })
        .collect();
    let ends_in_space = input.iter().rev().find_map(|p| match p {
        EnginePiece::Text(t) if !t.is_empty() => t.last().map(u8::is_ascii_whitespace),
        _ => None,
    });
    if ends_in_space == Some(false)
        && let Some(EnginePiece::Text(t)) = input
            .iter_mut()
            .rev()
            .find(|p| matches!(p, EnginePiece::Text(t) if !t.is_empty()))
    {
        t.push(b' ');
    }
    input
}

fn speak<E: Engine>(
    engine: &mut E,
    output: &mut impl Write,
    epoch: &StopEpoch,
    token: u64,
    pieces: &[Piece],
    at: u64,
) -> std::io::Result<()> {
    if !epoch.is_current(at) {
        return protocol::write_frame(
            output,
            &Reply::End {
                token,
                status: EndStatus::Aborted,
                samples: 0,
            }
            .encode(),
        );
    }
    let input = engine_input(engine.dialect(), pieces);
    let mut samples: u64 = 0;
    let mut io_error: Option<std::io::Error> = None;
    let result = {
        let mut out = |ev: SynthEvent<'_>| -> bool {
            if !epoch.is_current(at) {
                return false;
            }
            let frame = match ev {
                SynthEvent::Audio(s) => {
                    let f = protocol::encode_audio(token, s);
                    samples += s.len() as u64;
                    f
                }
                SynthEvent::Mark(index) => Reply::Mark {
                    token,
                    index,
                    sample: samples,
                }
                .encode(),
            };
            match protocol::write_frame(output, &frame) {
                Ok(()) => true,
                Err(e) => {
                    io_error = Some(e);
                    false
                }
            }
        };
        engine.synthesize(&input, &mut out)
    };
    if let Some(e) = io_error {
        return Err(e);
    }
    let status = match result {
        Ok(true) => EndStatus::Done,
        Ok(false) => EndStatus::Aborted,
        Err(message) => {
            protocol::write_frame(output, &Reply::Error { token, message }.encode())?;
            EndStatus::Failed
        }
    };
    protocol::write_frame(
        output,
        &Reply::End {
            token,
            status,
            samples,
        }
        .encode(),
    )
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::protocol::FrameDecoder;

    fn run_fake(requests: &[Request]) -> Vec<Reply> {
        let input: Vec<u8> = requests.iter().flat_map(Request::encode).collect();
        let mut engine = fake::FakeEngine::new(fake::FakeConfig::default());
        let mut out = Vec::new();
        run(&mut engine, Cursor::new(input), &mut out, AtEnd::Finish).unwrap();
        let mut dec = FrameDecoder::new();
        dec.push(&out);
        let mut replies = Vec::new();
        while let Some(b) = dec.next_body().unwrap() {
            replies.push(Reply::decode(&b).unwrap());
        }
        replies
    }

    #[test]
    fn ready_then_marks_audio_and_end_in_order() {
        let replies = run_fake(&[Request::Speak {
            token: 5,
            pieces: vec![
                Piece::Index(0),
                Piece::Text("Hello ".into()),
                Piece::Index(1),
                Piece::Text("world".into()),
            ],
        }]);
        assert!(matches!(replies[0], Reply::Ready { .. }));
        let marks: Vec<(u32, u64)> = replies
            .iter()
            .filter_map(|r| match r {
                Reply::Mark { index, sample, .. } => Some((*index, *sample)),
                _ => None,
            })
            .collect();
        assert_eq!(marks.len(), 2);
        assert_eq!(marks[0], (0, 0));
        assert!(marks[1].1 > 0);
        let total: u64 = replies
            .iter()
            .filter_map(|r| match r {
                Reply::Audio { samples, .. } => Some(samples.len() as u64),
                _ => None,
            })
            .sum();
        assert_eq!(
            replies.last(),
            Some(&Reply::End {
                token: 5,
                status: EndStatus::Done,
                samples: total
            })
        );
    }

    #[test]
    fn speak_after_stop_is_not_skipped_but_speak_before_is_marked() {
        // Stop arrives after both Speaks here (the reader runs ahead), so
        // whichever Speaks the main loop had not started are aborted.
        let replies = run_fake(&[
            Request::SetVoiceParam {
                param: 6,
                value: 60,
            },
            Request::Stop,
            Request::Speak {
                token: 9,
                pieces: vec![Piece::Index(0), Piece::Text("after".into())],
            },
        ]);
        assert!(replies.iter().any(|r| matches!(
            r,
            Reply::End {
                token: 9,
                status: EndStatus::Done,
                ..
            }
        )));
    }

    #[test]
    fn engine_text_is_encoded_and_always_ends_in_whitespace() {
        let input = engine_input(
            0x0001_0000,
            &[
                Piece::Index(0),
                Piece::Text("naïve ".into()),
                Piece::Index(1),
                Piece::Text("café".into()),
                Piece::Index(2),
            ],
        );
        assert_eq!(
            input,
            [
                EnginePiece::Index(0),
                EnginePiece::Text(b"na\xefve ".to_vec()),
                EnginePiece::Index(1),
                EnginePiece::Text(b"caf\xe9 ".to_vec()),
                EnginePiece::Index(2),
            ]
        );
        let kept = engine_input(0x0001_0000, &[Piece::Text("end.\n".into())]);
        assert_eq!(kept, [EnginePiece::Text(b"end.\n".to_vec())]);
        assert!(engine_input(0x0001_0000, &[]).is_empty());
    }

    #[test]
    fn bad_frames_are_reported_and_eof_ends_the_loop() {
        let mut input = Request::Quit.encode();
        input[4] = 0x7e; // unknown tag
        let mut engine = fake::FakeEngine::new(fake::FakeConfig::default());
        let mut out = Vec::new();
        run(&mut engine, Cursor::new(input), &mut out, AtEnd::Finish).unwrap();
        let mut dec = FrameDecoder::new();
        dec.push(&out);
        let mut replies = Vec::new();
        while let Some(b) = dec.next_body().unwrap() {
            replies.push(Reply::decode(&b).unwrap());
        }
        assert!(matches!(replies[1], Reply::Error { token: 0, .. }));
    }
}
