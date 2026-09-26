//! The host side of the protocol: the loop `textweaver-dectalk-host` runs.
//!
//! The loop is generic over [`Engine`], so the same code drives the real
//! DECtalk library ([`ffi::DectalkEngine`]) and the deterministic
//! [`fake::FakeEngine`] the backend's tests use.
//!
//! Threads: the shared engine host's reader thread ([`RequestReader`])
//! decodes requests from stdin. It handles `Stop` itself by bumping a stop
//! epoch; every `Speak` is stamped with the epoch at which it was read and
//! is skipped (ended `Aborted`) if a `Stop` followed it, and a `Stop` that
//! arrives while an utterance is being synthesized ends that utterance
//! `Aborted` before its audio goes out. The main thread owns the engine
//! and writes replies.

pub mod fake;
pub mod ffi;
pub mod input;

use std::io::{Read, Write};

use textweaver_enginehost::serve::{AtEnd, Incoming, RequestReader, StopEpoch};

pub use input::EnginePiece;

use crate::protocol::{self, EndStatus, Piece, Reply, Request};
use crate::voices::{self, Speaker};

/// What the engine reports at start-up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineInfo {
    /// Output sample rate in Hz.
    pub sample_rate: u32,
    /// Engine version, as far as it is known.
    pub version: String,
    /// `dectalk` or `fake`.
    pub engine: String,
}

/// The voice for the next utterances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The speaker.
    pub speaker: Speaker,
    /// Rate in words per minute (75..=600).
    pub rate: u16,
    /// Average pitch in Hz; 0 keeps the speaker's own.
    pub pitch_hz: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            speaker: Speaker::Paul,
            rate: voices::DEFAULT_RATE,
            pitch_hz: 0,
        }
    }
}

/// Something the engine produced during synthesis.
#[derive(Debug)]
pub enum SynthEvent<'a> {
    /// PCM samples.
    Audio(&'a [i16]),
    /// The mark before word `n` was reached (after all audio before it).
    Mark(u32),
}

/// A text-to-speech engine the host can drive.
pub trait Engine {
    /// Start-up facts.
    fn info(&mut self) -> EngineInfo;
    /// Synthesizes `pieces` with `settings`, calling `out` for every event
    /// in order. `out` returns false to abort. Returns `Ok(true)` when
    /// synthesis completed, `Ok(false)` when it was aborted.
    fn synthesize(
        &mut self,
        settings: &Settings,
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
    protocol::write_frame(
        output,
        &Reply::Ready {
            protocol: protocol::PROTOCOL_VERSION,
            sample_rate: info.sample_rate,
            version: info.version,
            engine: info.engine,
        }
        .encode(),
    )?;
    let reader = RequestReader::<Request>::spawn_with(input, "dectalk-host-reader", at_end)?;
    let epoch = reader.epoch().clone();
    let mut settings = Settings::default();
    while let Some(item) = reader.next() {
        match item {
            Incoming::Request(Request::Quit, _) => break,
            Incoming::Request(Request::Stop, _) => {}
            Incoming::Bad(message) => {
                protocol::write_frame(output, &Reply::Error { token: 0, message }.encode())?;
            }
            Incoming::Request(
                Request::SetVoice {
                    speaker,
                    rate,
                    pitch,
                },
                _,
            ) => match Speaker::from_index(speaker) {
                Some(speaker) => {
                    settings = Settings {
                        speaker,
                        rate: voices::rate(rate),
                        pitch_hz: if pitch == 0 {
                            0
                        } else {
                            pitch.clamp(voices::MIN_PITCH_HZ, voices::MAX_PITCH_HZ)
                        },
                    };
                }
                None => {
                    let message = format!("no DECtalk speaker {speaker}");
                    protocol::write_frame(output, &Reply::Error { token: 0, message }.encode())?;
                }
            },
            Incoming::Request(Request::Speak { token, pieces }, at) => {
                speak(engine, output, &epoch, &settings, token, &pieces, at)?;
            }
        }
    }
    Ok(())
}

/// Encodes pieces for DECtalk.
pub fn engine_input(pieces: &[Piece]) -> Vec<EnginePiece> {
    pieces
        .iter()
        .map(|p| match p {
            Piece::Text(s) => EnginePiece::Text(input::encode_text(s)),
            Piece::Index(i) => EnginePiece::Index(*i),
        })
        .collect()
}

fn speak<E: Engine>(
    engine: &mut E,
    output: &mut impl Write,
    epoch: &StopEpoch,
    settings: &Settings,
    token: u64,
    pieces: &[Piece],
    at: u64,
) -> std::io::Result<()> {
    let end = |status, samples| Reply::End {
        token,
        status,
        samples,
    };
    if !epoch.is_current(at) {
        return protocol::write_frame(output, &end(EndStatus::Aborted, 0).encode());
    }
    let input = engine_input(pieces);
    let mut samples: u64 = 0;
    let mut io_error: Option<std::io::Error> = None;
    let result = {
        let mut out = |ev: SynthEvent<'_>| -> bool {
            if !epoch.is_current(at) {
                return false;
            }
            let frame = match ev {
                SynthEvent::Audio(s) => {
                    samples += s.len() as u64;
                    protocol::encode_audio(token, s)
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
        engine.synthesize(settings, &input, &mut out)
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
    protocol::write_frame(output, &end(status, samples).encode())
}

/// Sends synthesized audio and marks in order: the audio before each mark,
/// then the mark, then the rest. `marks` are `(word, sample)` pairs in any
/// order; samples past the end of the audio are placed at its end. Audio
/// goes out in blocks of at most `block` samples. Returns false when `out`
/// asked to stop.
pub fn emit_in_order(
    samples: &[i16],
    marks: &mut [(u32, u64)],
    block: usize,
    out: &mut dyn FnMut(SynthEvent<'_>) -> bool,
) -> bool {
    marks.sort_by_key(|&(word, at)| (at, word));
    let block = block.max(1);
    let mut sent = 0usize;
    let mut send_until = |to: usize, out: &mut dyn FnMut(SynthEvent<'_>) -> bool| -> bool {
        while sent < to {
            let n = (to - sent).min(block);
            if !out(SynthEvent::Audio(&samples[sent..sent + n])) {
                return false;
            }
            sent += n;
        }
        true
    };
    for &(word, at) in marks.iter() {
        let at = usize::try_from(at).unwrap_or(usize::MAX).min(samples.len());
        if !send_until(at, out) || !out(SynthEvent::Mark(word)) {
            return false;
        }
    }
    send_until(samples.len(), out)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::protocol::FrameDecoder;

    fn run_fake(requests: &[Request]) -> Vec<Reply> {
        let input: Vec<u8> = requests.iter().flat_map(Request::encode).collect();
        let mut engine = fake::FakeEngine;
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
        assert!(matches!(
            &replies[0],
            Reply::Ready { engine, sample_rate: 11025, .. } if engine == "fake"
        ));
        let marks: Vec<(u32, u64)> = replies
            .iter()
            .filter_map(|r| match r {
                Reply::Mark { index, sample, .. } => Some((*index, *sample)),
                _ => None,
            })
            .collect();
        assert_eq!(marks, [(0, 0), (1, 6 * fake::SAMPLES_PER_BYTE as u64)]);
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
    fn set_voice_changes_the_next_utterance_and_bad_speakers_are_reported() {
        let speak = |token| Request::Speak {
            token,
            pieces: vec![Piece::Text("abcd".into())],
        };
        let replies = run_fake(&[
            speak(1),
            Request::SetVoice {
                speaker: 4,
                rate: 360,
                pitch: 0,
            },
            speak(2),
            Request::SetVoice {
                speaker: 9,
                rate: 180,
                pitch: 0,
            },
        ]);
        let len = |t| {
            replies
                .iter()
                .find_map(|r| match r {
                    Reply::End { token, samples, .. } if *token == t => Some(*samples),
                    _ => None,
                })
                .unwrap()
        };
        // Twice the rate, half the samples.
        assert_eq!(len(1), 2 * len(2));
        assert!(
            matches!(replies.last(), Some(Reply::Error { token: 0, message }) if message.contains("speaker 9"))
        );
    }

    #[test]
    fn failures_are_reported_on_their_utterance() {
        let replies = run_fake(&[Request::Speak {
            token: 3,
            pieces: vec![Piece::Text("please __fail__".into())],
        }]);
        let n = replies.len();
        assert!(matches!(&replies[n - 2], Reply::Error { token: 3, .. }));
        assert!(matches!(
            replies[n - 1],
            Reply::End {
                token: 3,
                status: EndStatus::Failed,
                ..
            }
        ));
    }

    #[test]
    fn bad_frames_are_reported_and_eof_ends_the_loop() {
        let mut input = Request::Quit.encode();
        input[4] = 0x7e;
        let mut engine = fake::FakeEngine;
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

    #[test]
    fn audio_and_marks_go_out_in_sample_order() {
        let samples: Vec<i16> = (0..10).collect();
        let mut marks = vec![(2, 7), (0, 0), (1, 3), (3, 99)];
        let mut got = Vec::new();
        let go = emit_in_order(&samples, &mut marks, 2, &mut |e| {
            got.push(match e {
                SynthEvent::Audio(s) => format!("a{}", s.len()),
                SynthEvent::Mark(m) => format!("m{m}"),
            });
            true
        });
        assert!(go);
        assert_eq!(
            got,
            ["m0", "a2", "a1", "m1", "a2", "a2", "m2", "a2", "a1", "m3"]
        );
        // Stopping part-way.
        let mut n = 0;
        let go = emit_in_order(&samples, &mut marks, 2, &mut |_| {
            n += 1;
            n < 3
        });
        assert!(!go);
        assert_eq!(n, 3);
    }
}
