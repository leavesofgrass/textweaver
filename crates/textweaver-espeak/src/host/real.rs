//! The real engine: libespeak-ng in retrieval mode, through the loader and
//! synthesis calls of `textweaver-speech` (its `espeak` module), so the
//! helper and the in-process backend share one set of FFI declarations,
//! one library search (the components folder first), and one mapping of
//! rate and pitch.
//!
//! libespeak-ng hands each block of audio to a callback while
//! `espeak_Synth` runs, on the calling thread. The engine therefore runs
//! the library on a worker thread of its own and forwards each block over
//! a channel, so the host's main thread writes it to the pipe at once.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};

use textweaver_core::{Pitch, Rate};
use textweaver_speech::backends::espeak;
use textweaver_speech::{VoiceParams, WordTiming};

use super::{Engine, EngineInfo, SynthEvent, VoiceSettings};
use crate::protocol::VoiceEntry;

enum Job {
    Voice(VoiceParams, Sender<Result<(), String>>),
    Speak {
        text: String,
        character: Option<char>,
        stop: Arc<AtomicBool>,
        blocks: Sender<Block>,
    },
}

enum Block {
    Audio(Vec<i16>, Vec<WordTiming>),
    Done(Result<bool, String>),
}

/// libespeak-ng on its worker thread.
#[derive(Debug)]
pub struct EspeakEngine {
    jobs: Sender<Job>,
    info: EngineInfo,
}

fn entry(v: textweaver_speech::Voice) -> VoiceEntry {
    VoiceEntry {
        gender: match v.gender.as_deref() {
            Some("male") => 1,
            Some("female") => 2,
            _ => 0,
        },
        id: v.id,
        name: v.name,
        languages: v.languages,
    }
}

impl EspeakEngine {
    /// Loads libespeak-ng (the one at `library` when given, else the first
    /// found: the components folder, `TEXTWEAVER_ESPEAK_LIBRARY`, the
    /// usual places) and starts it in retrieval mode.
    pub fn start(library: Option<std::path::PathBuf>) -> Result<Self, String> {
        let shown = library
            .as_ref()
            .map_or_else(|| "libespeak-ng".to_owned(), |p| p.display().to_string());
        if let Some(path) = library {
            espeak::choose_library(path);
        }
        let (jobs, rx) = mpsc::channel::<Job>();
        let (ready_tx, ready_rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("espeak-host-engine".into())
            .spawn(move || {
                let started =
                    espeak::retrieval_start().map(|rate| (rate, espeak::retrieval_voices()));
                let ok = started.is_ok();
                let _ = ready_tx.send(started);
                if ok {
                    work(&rx);
                }
            })
            .map_err(|e| e.to_string())?;
        let (sample_rate, voices) = ready_rx
            .recv()
            .map_err(|_| "the engine thread ended".to_owned())??;
        Ok(EspeakEngine {
            jobs,
            info: EngineInfo {
                sample_rate,
                version: shown,
                engine: "espeak-ng".into(),
                voices: voices.into_iter().map(entry).collect(),
            },
        })
    }
}

fn work(jobs: &Receiver<Job>) {
    while let Ok(job) = jobs.recv() {
        match job {
            Job::Voice(params, reply) => {
                let _ = reply.send(espeak::retrieval_params(&params));
            }
            Job::Speak {
                text,
                character,
                stop,
                blocks,
            } => {
                let tx = blocks.clone();
                let r = espeak::retrieval_synthesize(
                    &text,
                    character,
                    Box::new(move |wav: &[i16], words: &[WordTiming]| {
                        let _ = tx.send(Block::Audio(wav.to_vec(), words.to_vec()));
                        stop.load(Ordering::SeqCst)
                    }),
                );
                let _ = blocks.send(Block::Done(r));
            }
        }
    }
}

impl Engine for EspeakEngine {
    fn info(&mut self) -> EngineInfo {
        self.info.clone()
    }

    fn set_voice(&mut self, voice: &VoiceSettings) -> Result<(), String> {
        let params = VoiceParams {
            voice: (!voice.voice.is_empty()).then(|| voice.voice.clone()),
            rate: Rate::Wpm(voice.rate),
            pitch: Pitch::Semitones(voice.pitch),
            // The backend applies volume as a playback gain.
            ..VoiceParams::default()
        };
        let (tx, rx) = mpsc::channel();
        self.jobs
            .send(Job::Voice(params, tx))
            .map_err(|_| "the engine thread ended".to_owned())?;
        rx.recv()
            .map_err(|_| "the engine thread ended".to_owned())?
    }

    fn synthesize(
        &mut self,
        text: &str,
        character: bool,
        out: &mut dyn FnMut(SynthEvent<'_>) -> bool,
    ) -> Result<bool, String> {
        let mut chars = text.chars();
        let character = match (character, chars.next(), chars.next()) {
            (true, Some(c), None) => Some(c),
            _ => None,
        };
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, blocks) = mpsc::channel();
        self.jobs
            .send(Job::Speak {
                text: text.to_owned(),
                character,
                stop: Arc::clone(&stop),
                blocks: tx,
            })
            .map_err(|_| "the engine thread ended".to_owned())?;
        let rate = u64::from(self.info.sample_rate);
        let mut going = true;
        // Every block is read, even after `out` asked to stop, until the
        // engine says it is done: the next job must not start early.
        while let Ok(block) = blocks.recv() {
            match block {
                Block::Audio(wav, words) => {
                    if !going {
                        continue;
                    }
                    for w in words {
                        // Rounded up, so the playback clock (which rounds
                        // down) gives back exactly eSpeak NG's milliseconds.
                        let sample = (u64::from(w.audio_ms) * rate).div_ceil(1000);
                        going = going && out(SynthEvent::Word(w.byte_range, sample));
                    }
                    going = going && (wav.is_empty() || out(SynthEvent::Audio(&wav)));
                    if !going {
                        stop.store(true, Ordering::SeqCst);
                    }
                }
                Block::Done(r) => return r.map(|done| done && going),
            }
        }
        Err("the engine thread ended".into())
    }
}
