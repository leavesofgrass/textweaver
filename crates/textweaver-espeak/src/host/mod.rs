//! The host side of the protocol: the loop `textweaver-espeak-host` runs.
//!
//! The loop is generic over [`Engine`], so the same code drives the real
//! libespeak-ng ([`real::EspeakEngine`]) and the deterministic
//! [`fake::FakeEngine`] the backend's tests use.
//!
//! Threads: the shared engine host's reader thread ([`RequestReader`])
//! decodes requests from stdin. It handles `Stop` itself by bumping a stop
//! epoch; every `Speak` is stamped with the epoch at which it was read and
//! is skipped (ended `Aborted`) if a `Stop` followed it, and a `Stop` that
//! arrives while an utterance is being synthesized ends that utterance
//! `Aborted` at the next block of audio. The main thread writes replies.
//!
//! eSpeak NG synthesizes in small blocks and hands each one over as it is
//! made (retrieval mode), so every block goes out at once: the first
//! audio reaches textweaver after the first block, not after the whole
//! utterance, and a `Stop` takes effect within a block.

pub mod fake;
pub mod real;

use std::io::{Read, Write};
use std::ops::Range;

use textweaver_enginehost::serve::{AtEnd, Incoming, RequestReader, StopEpoch};

use crate::protocol::{self, EndStatus, Message, Reply, Request, VoiceEntry};

/// What the engine reports at start-up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineInfo {
    /// Output sample rate in Hz.
    pub sample_rate: u32,
    /// The library in use, as far as it is known.
    pub version: String,
    /// `espeak-ng` or `fake`.
    pub engine: String,
    /// The installed voices.
    pub voices: Vec<VoiceEntry>,
}

/// The voice for the next utterances.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceSettings {
    /// Voice identifier or name; empty for the default.
    pub voice: String,
    /// Rate in words per minute.
    pub rate: u16,
    /// Pitch offset in semitones.
    pub pitch: i8,
}

/// Something the engine produced during synthesis.
#[derive(Debug)]
pub enum SynthEvent<'a> {
    /// PCM samples.
    Audio(&'a [i16]),
    /// A word of the text (its byte range) starts at this sample of the
    /// utterance's audio.
    Word(Range<u32>, u64),
}

/// A text-to-speech engine the host can drive.
pub trait Engine {
    /// Start-up facts.
    fn info(&mut self) -> EngineInfo;
    /// Uses `voice` from the next utterance on. An unknown voice is an
    /// error and leaves the voice as it was.
    fn set_voice(&mut self, voice: &VoiceSettings) -> Result<(), String>;
    /// Synthesizes `text` (its one character by name when `character`),
    /// calling `out` for every event in order. `out` returns false to
    /// abort. Returns `Ok(true)` when synthesis completed, `Ok(false)`
    /// when it was aborted.
    fn synthesize(
        &mut self,
        text: &str,
        character: bool,
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
            voices: info.voices,
        }
        .encode(),
    )?;
    let reader = RequestReader::<Request>::spawn_with(input, "espeak-host-reader", at_end)?;
    let epoch = reader.epoch().clone();
    while let Some(item) = reader.next() {
        match item {
            Incoming::Request(Request::Quit, _) => break,
            Incoming::Request(Request::Stop, _) => {}
            Incoming::Bad(message) => {
                protocol::write_frame(output, &Reply::Error { token: 0, message }.encode())?;
            }
            Incoming::Request(Request::SetVoice { voice, rate, pitch }, _) => {
                if let Err(message) = engine.set_voice(&VoiceSettings { voice, rate, pitch }) {
                    protocol::write_frame(output, &Reply::Error { token: 0, message }.encode())?;
                }
            }
            Incoming::Request(
                Request::Speak {
                    token,
                    character,
                    text,
                },
                at,
            ) => speak(engine, output, &epoch, token, character, &text, at)?,
        }
    }
    Ok(())
}

fn speak<E: Engine>(
    engine: &mut E,
    output: &mut impl Write,
    epoch: &StopEpoch,
    token: u64,
    character: bool,
    text: &str,
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
                SynthEvent::Word(range, sample) => Reply::Word {
                    token,
                    range,
                    sample,
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
        engine.synthesize(text, character, &mut out)
    };
    if let Some(e) = io_error {
        return Err(e);
    }
    let status = match result {
        Ok(true) if epoch.is_current(at) => EndStatus::Done,
        Ok(_) => EndStatus::Aborted,
        Err(message) => {
            protocol::write_frame(output, &Reply::Error { token, message }.encode())?;
            EndStatus::Failed
        }
    };
    protocol::write_frame(output, &end(status, samples).encode())
}
