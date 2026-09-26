//! The shared engine host (ADR-0012): everything the out-of-process speech
//! engines have in common, so each engine crate keeps only what is its
//! own.
//!
//! Engines that cannot run inside textweaver's process (ETI-Eloquence
//! through ECI, ADR-0007; SAPI5 voices, ADR-0009) run in a helper process,
//! a *host*, that synthesizes into memory and streams PCM and word
//! positions back over a pipe. The backend in the main process plays the
//! audio and turns the positions into audio-clock word events. This crate
//! provides both sides of that arrangement:
//!
//! - [`protocol`]: the framed wire format, the messages every engine
//!   shares (Speak, Stop, Quit, Ready, Audio, Word, End, Error), and the
//!   version check;
//! - [`process`]: starting a host, reading its replies on a thread,
//!   noticing a crash or a hang, and shutting it down;
//! - [`Playback`] ([`playback`]): the playback client with the audio
//!   clock: utterance queue, `Started`/`Word`/`Finished`/`Cancelled`
//!   events per ADR-0003, native pause and resume, stop, and captures for
//!   `synthesize_to_file`;
//! - [`audio`]: the sample feed and the outputs (the audio device through
//!   rodio with the `playback` feature, or a silent timed output);
//! - [`wav`]: WAV writing;
//! - [`serve`]: the host side's request reader with its stop epoch, and a
//!   shared frame writer.
//!
//! The engine crates (`textweaver-eci`, `textweaver-sapi`) define their own
//! request and reply types on top of [`protocol`], their host binaries on
//! top of [`serve`], and their backends on top of [`process`] and
//! [`Playback`].

pub mod audio;
pub mod playback;
pub mod process;
pub mod protocol;
pub mod serve;
pub mod wav;

pub use audio::{AudioOutput, Feed, Player};
pub use playback::{Captured, Playback};
pub use process::{HostMsg, HostProcess};
pub use protocol::{EndStatus, Message, PROTOCOL_VERSION, ProtocolError};
